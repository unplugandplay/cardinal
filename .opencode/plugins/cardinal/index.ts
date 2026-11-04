// Cardinal harness adapter for OpenCode 2.
//
// Cardinal's gates are plain CLI commands that read a Claude Code hook event on
// stdin and refuse with exit code 2 (or a PreToolUse deny JSON). This plugin
// translates OpenCode's permission and tool events into that shape, so the same
// `cardinal run <tool>` commands guard an OpenCode + DeepSeek session.
//
// Wired:
//   .env                 -> default model      (OPENCODE_MODEL=<provider>/<model>)
//   read                 -> read-hint        (deny an oversized whole read once)
//   shell                -> review-gate check (refuse --no-verify, hooksPath, git add -A)
//   shell|edit           -> workspace-lock check (one live session per worktree)
//   edit|write           -> format-code after the edit
//   skill                -> review-gate record after a code-review skill
//   session created      -> workspace-lock start (with a per-session liveness process)
//   session deleted      -> workspace-lock release
//   session idle         -> lint-gate + language-hint (nudge once with the output)
//   session idle         -> transcript mirror under $XDG_DATA_HOME/cardinal/projects
//
// No Claude Code install is needed: the Claude-shaped event is built here, the
// per-session PID is a `sleep` process this plugin owns, and transcripts are
// written from OpenCode's own session context.

import { spawn, type ChildProcess } from "node:child_process"
import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs"
import { homedir } from "node:os"
import { join } from "node:path"
import { Plugin } from "@opencode/plugin"

const HOOK_TIMEOUT_MS = 600_000

// `.env` is the single place the provider/model choice lives.
function envModel(directory: string): { providerID: string; modelID: string } | undefined {
  try {
    const text = readFileSync(join(directory, ".env"), "utf8")
    for (const line of text.split(/\r?\n/)) {
      const match = line.match(/^\s*OPENCODE_MODEL\s*=\s*([^#\s]+)/)
      if (!match) continue
      const slash = match[1].indexOf("/")
      if (slash > 0) {
        return { providerID: match[1].slice(0, slash), modelID: match[1].slice(slash + 1) }
      }
    }
  } catch {
    // no .env: leave the model to OpenCode
  }
  return undefined
}

// The server may not share the shell's PATH; cargo installs to ~/.cargo/bin and
// some Cardinal tools need git >= 2.31 (Homebrew's, when it is present).
function cardinalBinary(): string {
  if (process.env.CARDINAL_BIN) return process.env.CARDINAL_BIN
  const cargo = join(homedir(), ".cargo", "bin", "cardinal")
  return existsSync(cargo) ? cargo : "cardinal"
}

function gatePath(): string {
  const brew = "/opt/homebrew/bin"
  return existsSync(join(brew, "git")) ? `${brew}:${process.env.PATH ?? ""}` : (process.env.PATH ?? "")
}

type HookResult = { code: number; stdout: string; stderr: string }

// One long-lived process per OpenCode session: workspace-lock uses a PID to tell
// whether another session still runs, and OpenCode sessions share one server.
const sleepers = new Map<string, ChildProcess>()

function ensureSleeper(sessionID: string | undefined): string | undefined {
  if (!sessionID) return undefined
  const existing = sleepers.get(sessionID)
  if (existing?.pid) return String(existing.pid)
  // Exits within a minute once the OpenCode server process is gone.
  const child = spawn("sh", ["-c", `while kill -0 ${process.pid} 2>/dev/null; do sleep 60; done`], {
    stdio: "ignore",
  })
  if (!child.pid) return undefined
  child.unref()
  sleepers.set(sessionID, child)
  return String(child.pid)
}

function dropSleeper(sessionID: string | undefined): void {
  if (!sessionID) return
  const child = sleepers.get(sessionID)
  if (!child) return
  child.kill()
  sleepers.delete(sessionID)
}

function sessionOf(event: unknown): string | undefined {
  if (typeof event !== "object" || event === null) return undefined
  const session = (event as Record<string, unknown>).session_id
  return typeof session === "string" && session.length > 0 ? session : undefined
}

function cardinal(
  directory: string,
  tool: string,
  args: string[],
  event: unknown,
  timeout = HOOK_TIMEOUT_MS,
): Promise<HookResult> {
  return new Promise((resolve) => {
    const session = sessionOf(event)
    const pid = ensureSleeper(session)
    const env: NodeJS.ProcessEnv = { ...process.env, PATH: gatePath(), CLAUDE_PROJECT_DIR: directory }
    if (pid) env.CLAUDE_PID = pid
    const child = spawn(cardinalBinary(), ["run", tool, ...args], {
      cwd: directory,
      env,
      stdio: ["pipe", "pipe", "pipe"],
    })
    let stdout = ""
    let stderr = ""
    const timer = setTimeout(() => child.kill("SIGKILL"), timeout)
    child.stdout.on("data", (chunk) => (stdout += String(chunk)))
    child.stderr.on("data", (chunk) => (stderr += String(chunk)))
    child.on("error", (error) => {
      clearTimeout(timer)
      console.error(`[cardinal] cannot run ${tool}: ${String(error)}`)
      resolve({ code: 127, stdout, stderr: String(error) })
    })
    child.on("close", (code) => {
      clearTimeout(timer)
      resolve({ code: code ?? 0, stdout, stderr })
    })
    child.stdin.end(JSON.stringify(event))
  })
}

function scalar(input: unknown, ...keys: string[]): string | undefined {
  if (typeof input !== "object" || input === null) return undefined
  for (const key of keys) {
    const value = (input as Record<string, unknown>)[key]
    if (typeof value === "string" && value.length > 0) return value
  }
  return undefined
}

function denyReason(stdout: string): string | undefined {
  try {
    const parsed = JSON.parse(stdout) as {
      hookSpecificOutput?: { permissionDecision?: string; permissionDecisionReason?: string }
    }
    if (parsed.hookSpecificOutput?.permissionDecision === "deny") {
      return parsed.hookSpecificOutput.permissionDecisionReason ?? "refused by Cardinal"
    }
  } catch {
    // not a deny JSON: allow
  }
  return undefined
}

type Part = { type?: string; text?: string; id?: string; name?: string; state?: { input?: unknown } }
type Message = {
  id?: string
  type?: string
  time?: { created?: number }
  text?: string
  content?: Part[]
  model?: { id?: string; providerID?: string }
  tokens?: { input?: number; output?: number; reasoning?: number; cache?: { read?: number; write?: number } }
}

function usageOf(message: Message): Record<string, unknown> {
  const tokens = message.tokens ?? {}
  return {
    input_tokens: tokens.input ?? 0,
    output_tokens: tokens.output ?? 0,
    output_tokens_details: { thinking_tokens: tokens.reasoning ?? 0 },
    cache_read_input_tokens: tokens.cache?.read ?? 0,
    cache_creation_input_tokens: tokens.cache?.write ?? 0,
  }
}

// The last assistant text of the session, in the shape language-hint judges.
function lastAssistantText(messages: readonly Message[]): string | undefined {
  for (let index = messages.length - 1; index >= 0; index -= 1) {
    const message = messages[index]
    if (message.type !== "assistant") continue
    const texts = (message.content ?? [])
      .filter((part) => part.type === "text" && typeof part.text === "string")
      .map((part) => part.text as string)
    if (texts.length > 0) return texts.join("\n\n")
  }
  return undefined
}

// A Claude-shaped JSONL transcript from OpenCode's own session context, so the
// evaluation tools read a session without Claude Code ever writing one.
function transcript(messages: readonly Message[], sessionID: string, cwd: string): string {
  const lines: string[] = []
  for (const message of messages) {
    const timestamp = new Date(message.time?.created ?? Date.now()).toISOString()
    if (message.type === "user") {
      lines.push(
        JSON.stringify({
          type: "user",
          sessionId: sessionID,
          cwd,
          timestamp,
          isSidechain: false,
          message: { role: "user", content: [{ type: "text", text: message.text ?? "" }] },
        }),
      )
      continue
    }
    if (message.type !== "assistant") continue
    const content = (message.content ?? []).flatMap((part) => {
      if (part.type === "text" && typeof part.text === "string") {
        return [{ type: "text", text: part.text }]
      }
      if (part.type === "tool") {
        return [{ type: "tool_use", id: part.id, name: part.name, input: part.state?.input ?? {} }]
      }
      return []
    })
    if (content.length === 0) continue
    lines.push(
      JSON.stringify({
        type: "assistant",
        sessionId: sessionID,
        cwd,
        timestamp,
        isSidechain: false,
        message: {
          id: message.id,
          role: "assistant",
          model: message.model ? `${message.model.providerID}/${message.model.id}` : undefined,
          usage: usageOf(message),
          content,
        },
      }),
    )
  }
  return lines.length > 0 ? `${lines.join("\n")}\n` : ""
}

function transcriptDir(directory: string, sessionID: string): { dir: string; file: string } {
  const base = process.env.XDG_DATA_HOME ?? join(homedir(), ".local", "share")
  const slug = directory.replaceAll("/", "-") || "-"
  const dir = join(base, "cardinal", "projects", slug)
  return { dir, file: join(dir, `${sessionID}.jsonl`) }
}

export default Plugin.define({
  id: "cardinal",
  async setup(ctx) {
    const directory = ctx.location.directory
    const nudged = new Set<string>()

    // The model comes from .env: pin it as the default when the provider and
    // model are available; an explicit --model still wins per run.
    const chosen = envModel(directory)
    if (chosen) {
      await ctx.model.transform((editor) => {
        const available = editor.list()
        if (available.some((model) => model.providerID === chosen.providerID && model.id === chosen.modelID)) {
          editor.default.set(chosen.providerID, chosen.modelID)
        }
      })
    }

    // One live session per worktree: a sleeper process per session supplies the
    // liveness PID, start takes the lock, release frees it.
    const events = new AbortController()
    void (async () => {
      for await (const event of ctx.event.subscribe({
        types: ["session.created", "session.deleted"],
        signal: events.signal,
      })) {
        const data = (event as unknown as { data?: { sessionID?: string } }).data
        const sessionID = data?.sessionID
        if (!sessionID) continue
        if (event.type === "session.created") {
          const result = await cardinal(directory, "workspace-lock", ["start"], {
            hook_event_name: "SessionStart",
            session_id: sessionID,
            cwd: directory,
          })
          if (result.stdout.trim()) {
            await ctx.session.synthetic({ sessionID, text: result.stdout.trim() })
          }
        } else {
          await cardinal(directory, "workspace-lock", ["release"], {
            hook_event_name: "SessionEnd",
            session_id: sessionID,
            cwd: directory,
          })
          dropSleeper(sessionID)
        }
      }
    })().catch((error) => console.error(`[cardinal] event stream: ${String(error)}`))

    // A user prompt starts a new turn: the lint nudge is available again.
    await ctx.session.hook("prompt", (event) => {
      nudged.delete(event.sessionID)
    })

    // PreToolUse equivalents: deny through the permission hook; its message is
    // shown to the model, which can adapt and continue.
    await ctx.permission.hook("evaluate", async (event) => {
      if (event.action === "shell" || event.action === "edit") {
        const lock = await cardinal(directory, "workspace-lock", ["check"], {
          hook_event_name: "PreToolUse",
          session_id: event.sessionID,
          cwd: directory,
        })
        if (lock.code === 2) {
          event.effect = "deny"
          event.message = lock.stderr.trim()
          return
        }
      }

      if (event.action === "shell") {
        for (const command of event.resources) {
          const result = await cardinal(directory, "review-gate", ["check"], {
            hook_event_name: "PreToolUse",
            tool_name: "Bash",
            tool_input: { command },
            cwd: directory,
            session_id: event.sessionID,
          })
          if (result.code === 2) {
            event.effect = "deny"
            event.message = result.stderr.trim()
            return
          }
        }
        return
      }

      if (event.action === "read") {
        for (const path of event.resources) {
          const result = await cardinal(directory, "read-hint", [], {
            hook_event_name: "PreToolUse",
            tool_name: "Read",
            tool_input: { file_path: path },
            cwd: directory,
            session_id: event.sessionID,
          })
          const reason = denyReason(result.stdout)
          if (reason) {
            event.effect = "deny"
            event.message = reason
            return
          }
        }
      }
    })

    // PostToolUse equivalents: format after an edit, record a finished review.
    await ctx.tool.hook("execute.after", async (event) => {
      if (event.status !== "completed") return
      const name = event.tool.split(".").pop() ?? event.tool

      if (name === "edit" || name === "write") {
        const path = scalar(event.input, "filePath", "file_path", "path")
        if (!path) return
        await cardinal(directory, "format-code", [], {
          hook_event_name: "PostToolUse",
          tool_name: name === "write" ? "Write" : "Edit",
          tool_input: { file_path: path },
          cwd: directory,
          session_id: event.sessionID,
        })
        return
      }

      if (name === "skill") {
        const skill = scalar(event.input, "name", "skill", "id")
        if (!skill) return
        await cardinal(directory, "review-gate", ["record"], {
          hook_event_name: "PostToolUse",
          tool_name: "Skill",
          tool_input: { skill },
          tool_response: { success: true },
          cwd: directory,
          session_id: event.sessionID,
        })
      }
    })

    // Stop-hook equivalents and the transcript mirror. On a failure, inject the
    // output once so the agent fixes it; the next idle lets go.
    await ctx.session.hook("experimental.idle", async (event) => {
      const messages = (await ctx.session.context({ sessionID: event.sessionID })) as unknown as Message[]
      const reply = lastAssistantText(messages)
      const nudgedAlready = nudged.has(event.sessionID)

      const lint = await cardinal(directory, "lint-gate", [], {
        hook_event_name: "Stop",
        cwd: directory,
        stop_hook_active: nudgedAlready,
        session_id: event.sessionID,
      })
      const language = await cardinal(directory, "language-hint", [], {
        hook_event_name: "Stop",
        cwd: directory,
        stop_hook_active: nudgedAlready,
        last_assistant_message: reply,
        session_id: event.sessionID,
      })

      try {
        const { dir, file } = transcriptDir(directory, event.sessionID)
        mkdirSync(dir, { recursive: true })
        const text = transcript(messages, event.sessionID, directory)
        if (text) writeFileSync(file, text)
      } catch (error) {
        console.error(`[cardinal] transcript mirror: ${String(error)}`)
      }

      if (nudgedAlready) return
      const reasons = [lint, language]
        .filter((result) => result.code === 2)
        .map((result) => result.stderr.trim())
        .filter(Boolean)
      if (reasons.length > 0) {
        nudged.add(event.sessionID)
        await ctx.session.synthetic({
          sessionID: event.sessionID,
          text: reasons.join("\n\n"),
        })
      }
    })

    return () => {
      events.abort()
      for (const sessionID of [...sleepers.keys()]) dropSleeper(sessionID)
    }
  },
})
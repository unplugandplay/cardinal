// Cardinal harness adapter for OpenCode 2 — plain JS, no build.
//
// It sends a neutral event to the Nim CLI (`nim/bin/cardinal-nim run <tool>`):
//   {action, tool?, input?, session, cwd, stop_active?, last_message?, success?}
// The CLI answers 0 (pass), 2 (refusal) or 3 (tool/usage error).
//
// Wired:
//   .env                    -> default model (OPENCODE_MODEL=<provider>/<model>)
//   read                    -> read-hint
//   shell                   -> review-gate check + workspace-lock check
//   edit                    -> workspace-lock check
//   edit|write (after)      -> format-code
//   skill (after)           -> review-gate record
//   session created/deleted -> workspace-lock start/release
//   session idle            -> lint-gate + language-hint, once per turn
//   session idle            -> transcript mirror under $XDG_DATA_HOME/cardinal/projects
//
// No Claude Code install is needed; the per-session liveness PID is a small
// shell process this plugin owns.

import { spawn } from "node:child_process"
import { appendFileSync, existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs"
import { homedir } from "node:os"
import { join } from "node:path"
import { Plugin } from "@opencode/plugin"

const HOOK_TIMEOUT_MS = 600_000
const DBG = "/tmp/cardinal-js.log"
function dbg(line) { try { appendFileSync(DBG, `${new Date().toISOString()} ${line}\n`) } catch {} }

function envModel(directory) {
  try {
    const text = readFileSync(join(directory, ".env"), "utf8")
    for (const line of text.split(/\r?\n/)) {
      const match = line.match(/^\s*OPENCODE_MODEL\s*=\s*([^#\s]+)/)
      if (!match) continue
      const slash = match[1].indexOf("/")
      if (slash > 0) return { providerID: match[1].slice(0, slash), modelID: match[1].slice(slash + 1) }
    }
  } catch {
    // no .env: leave the model to OpenCode
  }
  return undefined
}

function cardinalBinary(directory) {
  const candidates = [
    process.env.CARDINAL_BIN,
    join(directory, "nim", "bin", "cardinal-nim"),
    join(homedir(), ".local", "bin", "cardinal-nim"),
  ].filter(Boolean)
  for (const candidate of candidates) {
    if (existsSync(candidate)) return candidate
  }
  return "cardinal-nim"
}

function gatePath() {
  const brew = "/opt/homebrew/bin"
  return existsSync(join(brew, "git")) ? `${brew}:${process.env.PATH ?? ""}` : (process.env.PATH ?? "")
}

const sleepers = new Map()

function ensureSleeper(session) {
  if (!session) return undefined
  const existing = sleepers.get(session)
  if (existing?.pid) return String(existing.pid)
  // Exits within a minute once the OpenCode server process is gone.
  const child = spawn("sh", ["-c", `while kill -0 ${process.pid} 2>/dev/null; do sleep 60; done`], {
    stdio: "ignore",
  })
  if (!child.pid) return undefined
  child.unref()
  sleepers.set(session, child)
  return String(child.pid)
}

function dropSleeper(session) {
  const child = sleepers.get(session)
  if (!child) return
  child.kill()
  sleepers.delete(session)
}

function runGate(directory, tool, args, event, timeout = HOOK_TIMEOUT_MS) {
  return new Promise((resolve) => {
    const pid = ensureSleeper(event?.session)
    const env = { ...process.env, PATH: gatePath(), CLAUDE_PROJECT_DIR: directory }
    if (pid) env.CLAUDE_PID = pid
    const child = spawn(cardinalBinary(directory), ["run", tool, ...args], {
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
      resolve({ code: 127, stdout, stderr: String(error) })
    })
    child.on("close", (code) => {
      clearTimeout(timer)
      dbg(`gate ${tool} code=${code}`)
      resolve({ code: code ?? 0, stdout, stderr })
    })
    child.stdin.end(JSON.stringify(event ?? {}))
  })
}

function scalar(input, ...keys) {
  if (typeof input !== "object" || input === null) return undefined
  for (const key of keys) {
    const value = input[key]
    if (typeof value === "string" && value.length > 0) return value
  }
  return undefined
}

function denyReason(stdout) {
  try {
    const parsed = JSON.parse(stdout)
    if (parsed.hookSpecificOutput?.permissionDecision === "deny") {
      return parsed.hookSpecificOutput.permissionDecisionReason ?? "refused by Cardinal"
    }
  } catch {
    // not a deny JSON: allow
  }
  return undefined
}

function lastAssistantText(messages) {
  for (let index = messages.length - 1; index >= 0; index -= 1) {
    const message = messages[index]
    if (message.type !== "assistant") continue
    const texts = (message.content ?? [])
      .filter((part) => part.type === "text" && typeof part.text === "string")
      .map((part) => part.text)
    if (texts.length > 0) return texts.join("\n\n")
  }
  return undefined
}

function usageOf(message) {
  const tokens = message.tokens ?? {}
  return {
    input_tokens: tokens.input ?? 0,
    output_tokens: tokens.output ?? 0,
    output_tokens_details: { thinking_tokens: tokens.reasoning ?? 0 },
    cache_read_input_tokens: tokens.cache?.read ?? 0,
    cache_creation_input_tokens: tokens.cache?.write ?? 0,
  }
}

function transcript(messages, session, cwd) {
  const lines = []
  for (const message of messages) {
    const timestamp = new Date(message.time?.created ?? Date.now()).toISOString()
    if (message.type === "user") {
      lines.push(
        JSON.stringify({
          type: "user",
          sessionId: session,
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
      if (part.type === "text" && typeof part.text === "string") return [{ type: "text", text: part.text }]
      if (part.type === "tool") return [{ type: "tool_use", id: part.id, name: part.name, input: part.state?.input ?? {} }]
      return []
    })
    if (content.length === 0) continue
    lines.push(
      JSON.stringify({
        type: "assistant",
        sessionId: session,
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

function mirror(directory, session, messages) {
  const base = process.env.XDG_DATA_HOME ?? join(homedir(), ".local", "share")
  const slug = directory.replaceAll("/", "-") || "-"
  const dir = join(base, "cardinal", "projects", slug)
  mkdirSync(dir, { recursive: true })
  const text = transcript(messages, session, directory)
  if (text) writeFileSync(join(dir, `${session}.jsonl`), text)
}

export default Plugin.define({
  id: "cardinal",
  async setup(ctx) {
    const directory = ctx.location.directory
    dbg(`setup ${directory}`)
    const nudged = new Set()

    const chosen = envModel(directory)
    if (chosen) {
      await ctx.model.transform((editor) => {
        const available = editor.list()
        if (available.some((model) => model.providerID === chosen.providerID && model.id === chosen.modelID)) {
          editor.default.set(chosen.providerID, chosen.modelID)
        }
      })
    }

    const events = new AbortController()
    void (async () => {
      for await (const event of ctx.event.subscribe({
        types: ["session.created", "session.deleted"],
        signal: events.signal,
      })) {
        const session = event.data?.sessionID
        if (!session) continue
        if (event.type === "session.created") {
          const result = await runGate(directory, "workspace-lock", ["start"], {
            action: "start",
            session,
            cwd: directory,
          })
          if (result.stdout.trim()) await ctx.session.synthetic({ sessionID: session, text: result.stdout.trim() })
        } else {
          await runGate(directory, "workspace-lock", ["release"], {
            action: "stop",
            session,
            cwd: directory,
          })
          dropSleeper(session)
        }
      }
    })().catch((error) => console.error(`[cardinal] event stream: ${String(error)}`))

    await ctx.session.hook("prompt", (event) => {
      nudged.delete(event.sessionID)
    })

    await ctx.permission.hook("evaluate", async (event) => {
      dbg(`permission ${event.action}`)
      if (event.action === "shell" || event.action === "edit") {
        const lock = await runGate(directory, "workspace-lock", ["check"], {
          action: "lock",
          session: event.sessionID,
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
          const result = await runGate(directory, "review-gate", ["check"], {
            action: "shell",
            input: { command },
            session: event.sessionID,
            cwd: directory,
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
        dbg(`read resources ${JSON.stringify(event.resources)}`)
        for (const path of event.resources) {
          const result = await runGate(directory, "read-hint", [], {
            action: "read",
            input: { file_path: path },
            session: event.sessionID,
            cwd: directory,
          })
          dbg(`read-hint out=${result.stdout.slice(0, 160)} err=${result.stderr.slice(0, 80)}`)
          const reason = denyReason(result.stdout)
          if (reason) {
            event.effect = "deny"
            event.message = reason
            return
          }
        }
      }
    })

    await ctx.tool.hook("execute.after", async (event) => {
      if (event.status !== "completed") return
      const name = event.tool.split(".").pop() ?? event.tool

      if (name === "edit" || name === "write") {
        const path = scalar(event.input, "filePath", "file_path", "path")
        if (!path) return
        await runGate(directory, "format-code", [], {
          action: "format",
          input: { file_path: path },
          session: event.sessionID,
          cwd: directory,
        })
        return
      }

      if (name === "skill") {
        const skill = scalar(event.input, "name", "skill", "id")
        if (!skill) return
        await runGate(directory, "review-gate", ["record"], {
          action: "skill",
          input: { skill },
          success: true,
          session: event.sessionID,
          cwd: directory,
        })
      }
    })

    await ctx.session.hook("experimental.idle", async (event) => {
      const messages = await ctx.session.context({ sessionID: event.sessionID })
      const reply = lastAssistantText(messages)
      const nudgedAlready = nudged.has(event.sessionID)

      const lint = await runGate(directory, "lint-gate", [], {
        action: "idle",
        stop_active: nudgedAlready,
        session: event.sessionID,
        cwd: directory,
      })
      const language = await runGate(directory, "language-hint", [], {
        action: "idle",
        stop_active: nudgedAlready,
        last_message: reply,
        session: event.sessionID,
        cwd: directory,
      })

      try {
        mirror(directory, event.sessionID, messages)
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
        await ctx.session.synthetic({ sessionID: event.sessionID, text: reasons.join("\n\n") })
      }
    })

    return () => {
      events.abort()
      for (const session of [...sleepers.keys()]) dropSleeper(session)
    }
  },
})
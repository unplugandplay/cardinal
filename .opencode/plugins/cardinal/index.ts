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
//   edit|write           -> format-code after the edit
//   skill                -> review-gate record after a code-review skill
//   session idle         -> lint-gate        (nudge once with the lint output)
//
// Not wired (needs a Cardinal change or a transcript reader):
//   workspace-lock  — requires CLAUDE_PID, absent in OpenCode; all sessions
//                     share the server process, so liveness cannot distinguish them
//   language-hint   — reads Claude Code transcripts, which OpenCode does not write
//   evaluation      — session-metrics, rule-breaches, delivered-quality, ...

import { spawn } from "node:child_process"
import { existsSync, readFileSync } from "node:fs"
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

// The server may not share the shell's PATH; cargo installs to ~/.cargo/bin.
function cardinalBinary(): string {
  if (process.env.CARDINAL_BIN) return process.env.CARDINAL_BIN
  const cargo = join(homedir(), ".cargo", "bin", "cardinal")
  return existsSync(cargo) ? cargo : "cardinal"
}

type HookResult = { code: number; stdout: string; stderr: string }

function cardinal(
  directory: string,
  tool: string,
  args: string[],
  event: unknown,
  timeout = HOOK_TIMEOUT_MS,
): Promise<HookResult> {
  return new Promise((resolve) => {
    const child = spawn(cardinalBinary(), ["run", tool, ...args], {
      cwd: directory,
      env: { ...process.env, CLAUDE_PROJECT_DIR: directory },
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

    // A user prompt starts a new turn: the lint nudge is available again.
    await ctx.session.hook("prompt", (event) => {
      nudged.delete(event.sessionID)
    })

    // PreToolUse equivalents: deny through the permission hook; its message is
    // shown to the model, which can adapt and continue.
    await ctx.permission.hook("evaluate", async (event) => {
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
        })
      }
    })

    // Stop-hook equivalent: before the session goes idle, run the lint. On a
    // failure, inject its output once so the agent fixes it; the next idle lets
    // go, mirroring Cardinal's stop_hook_active retry limit.
    await ctx.session.hook("experimental.idle", async (event) => {
      const result = await cardinal(directory, "lint-gate", [], {
        hook_event_name: "Stop",
        cwd: directory,
        stop_hook_active: nudged.has(event.sessionID),
      })
      if (result.code === 2 && !nudged.has(event.sessionID)) {
        nudged.add(event.sessionID)
        await ctx.session.synthetic({
          sessionID: event.sessionID,
          text: result.stderr.trim(),
        })
      }
    })
  },
})

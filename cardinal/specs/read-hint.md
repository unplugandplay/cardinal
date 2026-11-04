---
id: RDH
status: accepted
adrs: [0005]
---

# Read hint

## Intent

The hook `cardinal run read-hint` (a `PreToolUse` hook on `Read`) sends the agent back once before it reads a large
file whole, naming the size and the range options, so the session's own results over the threshold of rule
`oversized-result` (BRE-19, `specs/rule-breaches.md`) fall. A repeat of the same read goes through: a file may be
needed whole.

## Requirements

- **RDH-1** (deterministic) On a `PreToolUse` event of tool `Read` whose `tool_input` has neither `offset` nor `limit`, naming with `file_path` a readable regular file of at most 262,144 bytes (the size above which Claude Code refuses a whole read itself) whose estimated result holds more than 25,000 bytes (the threshold of BRE-19), that the event's session has not been sent back from yet, the hook must print on stdout one JSON object whose `hookSpecificOutput` holds `hookEventName` `PreToolUse`, `permissionDecision` `deny` and a `permissionDecisionReason` naming that estimate in bytes, the options `offset` and `limit`, the tool `Grep`, and that the same call repeated reads the file whole, then exit 0.
- **RDH-2** (deterministic) The hook must exit 0 and print nothing on a `Read` event that has an `offset` or a `limit`, or names a file whose estimated result holds at most 25,000 bytes, a file of more than 262,144 bytes, a missing or unreadable file, or a file whose extension is `png`, `jpg`, `jpeg`, `gif`, `webp`, `pdf` or `ipynb` (case ignored).
- **RDH-3** (deterministic) After sending the session back from a file (RDH-1), the hook must exit 0 and print nothing on every later whole `Read` of that file by the same `session_id`.
- **RDH-4** (deterministic) The hook must exit 0 and print nothing on any event but a `PreToolUse` of tool `Read`, on an event that is not valid JSON or has no `session_id`, and when it cannot record that it sent the session back.
- **RDH-5** (deterministic) The hook must record the files it sent each session back from in one file per `session_id`, in a directory `cardinal-read-hint` of `XDG_RUNTIME_DIR` when it is set, else `cardinal-read-hint-<USER>` of the system's temporary directory (`TMPDIR`), created with mode 0700, and pass (RDH-4) when that directory lets its group or others in.
- **RDH-6** (deterministic) The source of Cardinal must declare the hook in `entries/.claude/settings.json` as `PreToolUse` matcher `Read` command `cardinal run read-hint`, and the feature `read-hint` made of that one entry and no file.
- **RDH-7** (deterministic) The estimated result of RDH-1 must be the bytes of the first 2,000 lines of the file, each cut at 2,000 characters, plus 7 bytes per line for the line number Claude Code prefixes.

## Scenarios

- (RDH-1) Given a file of 300 lines of 100 bytes, when the agent reads it with no `offset` nor `limit`, then the call
  is denied with a reason naming 32100 bytes (RDH-7), `offset`, `limit` and `Grep`; (RDH-3) when it repeats the call, it goes through.
- (RDH-2) Given a file whose estimate is at most 25,000 bytes, one of 300 lines read with `limit` 200, a minified
  one-line file of 200 KB (Claude Code cuts its line at 2,000 characters) or a log of 400 KB (Claude Code refuses it
  whole), when the agent reads it, then the hook prints nothing.
- (RDH-3) Given a session sent back from `a.rs`, when a second session reads `a.rs` whole, then it is sent back too.
- (RDH-2) Given a screenshot `shot.PNG` of 200 KB, when the agent reads it, then the hook prints nothing: Claude Code
  renders it as an image, not text.

## Non-goals

- A `cat`, `head` or `sed` of a large file through `Bash`: measured in its issue before any parsing.
- Bounding a read the agent repeats: the repeat says the whole file is needed.
- The exact bytes Claude Code would return: RDH-7 is an estimate, read before the call.
- Telling apart two spellings of one path: the hint asks to repeat the same call.
- Removing the records: they live in a runtime or temporary directory the system clears.

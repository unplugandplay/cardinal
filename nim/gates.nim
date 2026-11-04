## The gates, ported from the Rust tools with the same stdin contract and exit
## codes: 0 pass, 2 refusal, 3 tool/usage error.
##
## Ported: review-gate (check/record/native), lint-gate, read-hint,
## workspace-lock (check/start/release), format-code.
## Deferred: language-hint (it needs the word lists).

import std/[algorithm, json, os, osproc, posix, sequtils, streams, strutils, tables]

import config, metrics, sha256, shell, workflow

const
  FrenchWords = [
    "le", "la", "les", "un", "une", "des", "du", "de", "et", "est", "que", "qui", "dans", "pour",
    "pas", "sur", "avec", "ce", "cette", "il", "elle", "je", "tu", "nous", "vous", "au", "aux",
    "ou", "mais", "par", "sont", "fait", "sans", "comme", "se", "ne", "en", "donc", "puis",
  ]
  EnglishWords = [
    "the", "and", "is", "are", "that", "which", "in", "for", "not", "with", "this", "it", "i",
    "you", "we", "of", "to", "be", "by", "was", "but", "at", "from", "have", "has", "so", "then",
    "will", "can",
  ]
  MinWords = 20
  EmptyTree = "4b825dc642cb6eb9a060e54bf8d69288fbee4904"
  ReviewMarker = "claude-code-review.json"
  ReviewWaiver = "claude-review-waived"
  LockFile = "cardinal-session"
  LintStamp = "claude-lint-pass"
  ReadCap = 262_144
  Oversized = 25_000
  ReadLines = 2_000
  LineChars = 2_000
  NumberBytes = 7
  Rendered = ["png", "jpg", "jpeg", "gif", "webp", "pdf", "ipynb"]

type
  Holder = object
    session: string
    pid: string

proc wildcard(pattern, text: string): bool =
  if pattern.len == 0:
    return text.len == 0
  case pattern[0]
  of '*':
    for i in 0 .. text.len:
      if wildcard(pattern[1 .. ^1], text[i .. ^1]):
        return true
    false
  of '?':
    text.len > 0 and wildcard(pattern[1 .. ^1], text[1 .. ^1])
  else:
    text.len > 0 and text[0] == pattern[0] and wildcard(pattern[1 .. ^1], text[1 .. ^1])

proc actsOn(files: seq[string], path: string): bool =
  let patterns = if files.len == 0: @["*.rs"] else: files
  for pattern in patterns:
    let name = path.extractFilename
    if wildcard(pattern, path) or wildcard(pattern, name):
      return true
  false

proc changedFiles(cwd: string; lintFiles: seq[string]): seq[string] =
  let status = git(cwd, @["status", "--porcelain", "-z", "--untracked-files=all"])
  var entries = status.split('\0')
  var index = 0
  while index < entries.len:
    let entry = entries[index]
    inc index
    if entry.len < 4:
      continue
    let code = entry[0 .. 1]
    let path = entry[3 .. ^1]
    if (code[0] == 'R' or code[0] == 'C') and index < entries.len:
      inc index # skip the source path of a rename/copy
    if actsOn(lintFiles, path):
      result.add(path)

proc lintState(cwd, command: string; files: seq[string]): string =
  var text = command & "\0" & git(cwd, @["rev-parse", "HEAD"])
  for file in files:
    text.add("\0" & file & "\0")
    try: text.add(readFile(cwd / file)) except CatchableError: discard
  sha256Hex(text)

proc lintGate(event: JsonNode): int =
  if event{"stop_hook_active"}.getBool(false):
    return 0
  let cwd = block:
    let fromEvent = event{"cwd"}.getStr("")
    if fromEvent.len > 0: fromEvent else: getEnv("CLAUDE_PROJECT_DIR", ".")
  let root = findProjectRoot(cwd)
  if root.len == 0:
    return 0
  let project = readProject(root)
  if project.lintDeclared and project.lintCommand.len == 0:
    return 0
  let files = changedFiles(cwd, project.lintFiles)
  if files.len == 0:
    return 0
  let state = lintState(cwd, project.lintCommand, files)
  let stamp = gitPath(cwd, LintStamp)
  if stamp.len > 0 and fileExists(stamp):
    try:
      if readFile(stamp) == state:
        return 0
    except CatchableError: discard

  var output: string
  var passed: bool
  if project.lintDeclared:
    let run = execCapture("sh", @["-c", project.lintCommand], cwd, "")
    output = run.output
    passed = run.code == 0
  else:
    let run = execCapture("cargo", @["clippy", "--workspace", "--all-targets",
                                     "--message-format=short", "--", "-D", "warnings"], cwd, "")
    output = run.output
    passed = run.code == 0
  if passed:
    if stamp.len > 0:
      writeFile(stamp, lintState(cwd, project.lintCommand, changedFiles(cwd, project.lintFiles)))
    return 0

  if project.lintDeclared:
    stderr.writeLine("`" & project.lintCommand & "` reports problems; fix them (the project must stay warning-free):")
  else:
    stderr.writeLine("cargo clippy reports problems; fix them (the project must stay warning-free):")
  var printed = 0
  for line in output.splitLines:
    let text = line.strip()
    if text.len == 0:
      continue
    if not project.lintDeclared:
      let first = text.splitWhitespace[0]
      if first in ["Compiling", "Checking", "Updating", "Downloading", "Downloaded",
                   "Locking", "Adding", "Blocking", "Fresh", "Finished"] or
         text.contains("could not compile"):
        continue
    stderr.writeLine(text)
    inc printed
    if printed >= 40:
      break
  2

proc snapshot(cwd: string): tuple[head, hash: string, files: seq[string]] =
  result.head = git(cwd, @["rev-parse", "-q", "--verify", "HEAD"]).strip()
  let base = if result.head.len == 0: EmptyTree else: "HEAD"
  var content = git(cwd, @["diff", base, "--binary", "--no-color", "--no-ext-diff", "--no-textconv"])
  result.files = git(cwd, @["diff", base, "--name-only", "--no-renames", "-z"])
    .split('\0').filterIt(it.len > 0)
  let untracked = git(cwd, @["ls-files", "--others", "--exclude-standard", "-z"])
    .split('\0').filterIt(it.len > 0)
  for name in untracked:
    let hash = git(cwd, @["hash-object", "--no-filters", "--", name]).strip()
    content.add("\0" & name & "\0" & hash & "\0")
    result.files.add(name)
  result.files.sort()
  result.hash = git(cwd, @["hash-object", "--stdin"], content).strip()

proc refuseReview(why: string): int =
  stderr.writeLine("Refused: " & why & ".")
  stderr.writeLine("Run /code-review low on the current diff with the Skill tool, fix what it finds, then commit.")
  2

proc skipsHooks(command: string): bool =
  let at = command.find("git commit")
  if at < 0:
    return false
  var rest = command[at + "git commit".len .. ^1]
  for stop in [";", "&&", "||", "|", "\n"]:
    let cut = rest.find(stop)
    if cut >= 0:
      rest = rest[0 ..< cut]
  if rest.contains("--no-verify"):
    return true
  for token in rest.splitWhitespace:
    if token.len < 2 or token[0] != '-' or token.startsWith("--"):
      continue
    if token.contains(':') or token.contains('='):
      continue
    var letters = true
    for letter in token[1 .. ^1]:
      if not letter.isAlphaAscii:
        letters = false
    if letters and token.contains('n'):
      return true
  false

proc stagesEverything(command: string): bool =
  let at = command.find("git add")
  if at < 0:
    return false
  var rest = command[at + 7 .. ^1]
  for stop in [";", "&&", "||", "|", "\n"]:
    let cut = rest.find(stop)
    if cut >= 0:
      rest = rest[0 ..< cut]
  for token in rest.splitWhitespace:
    if token in ["-A", "--all", ".", ":/", "*", "-u", "--update"]:
      return true
  false

proc reviewCheck(event: JsonNode): int =
  let command = event{"tool_input", "command"}.getStr("")
  if command.len == 0:
    return 0
  if command.contains("core.hooksPath"):
    stderr.writeLine("Refused: setting core.hooksPath skips the native pre-commit review hook that `cardinal sync` installs.")
    stderr.writeLine("Commit without it, or ask the user to waive the review.")
    return 2
  if skipsHooks(command):
    stderr.writeLine("Refused: --no-verify or -n skips the native pre-commit review hook that `cardinal sync` installs.")
    stderr.writeLine("Commit without it, or ask the user to waive the review.")
    return 2
  if stagesEverything(command):
    stderr.writeLine("Refused: `git add` of every file stages personal and unrelated files with the deliverable.")
    stderr.writeLine("Add the files of the deliverable by name.")
    return 2
  0

proc reviewRecord(event: JsonNode; cwd: string): int =
  let skill = event{"tool_input", "skill"}.getStr("")
  let succeeded = event{"tool_response", "success"}.getBool(false)
  if skill.rsplit(':')[^1] != "code-review" or not succeeded:
    return 0
  let marker = gitPath(cwd, ReviewMarker)
  if marker.len == 0:
    return 0
  let snap = snapshot(cwd)
  writeFile(marker, $(%*{"head": snap.head, "hash": snap.hash, "files": %snap.files}))
  0

proc reviewNative(cwd: string): int =
  let snap = snapshot(cwd)
  let marker = gitPath(cwd, ReviewMarker)
  if marker.len == 0:
    return 0
  let stamp = if snap.head.len == 0: "no-head" else: snap.head
  let waiverPath = gitPath(cwd, ReviewWaiver)
  var waiver = false
  if waiverPath.len > 0 and fileExists(waiverPath):
    var used = ""
    try: used = readFile(waiverPath).strip() except CatchableError: discard
    if used.len > 0 and used != stamp:
      removeFile(waiverPath)
    else:
      waiver = true
  if snap.files.len == 0:
    return 0
  if waiver:
    try:
      writeFile(waiverPath, stamp)
      return 0
    except CatchableError:
      discard
  if not fileExists(marker):
    return refuseReview("no code review has run in this project")
  var node: JsonNode
  try:
    node = parseJson(readFile(marker))
  except CatchableError, ValueError:
    return refuseReview("no code review has run in this project")
  if node{"head"}.getStr("") != snap.head:
    return refuseReview("the last code review predates the last commit")
  if node{"hash"}.getStr("") == snap.hash:
    return 0
  var reviewed: seq[string]
  if node{"files"}.kind == JArray:
    for item in node{"files"}:
      reviewed.add(item.getStr(""))
  for file in snap.files:
    if file notin reviewed:
      return refuseReview("the diff holds files the last code review never saw")
  0

proc readRecordDir(): string =
  var dir = ""
  let runtime = getEnv("XDG_RUNTIME_DIR")
  if runtime.len > 0:
    dir = runtime / "cardinal-read-hint"
  else:
    dir = getTempDir() / ("cardinal-read-hint-" & getEnv("USER"))
  if dirExists(dir):
    if getFilePermissions(dir) == {fpUserRead, fpUserWrite, fpUserExec}:
      return dir
    return ""
  createDir(dir)
  setFilePermissions(dir, {fpUserRead, fpUserWrite, fpUserExec})
  dir

proc estimate(text: string): int =
  var lines = 0
  var split = text.splitLines
  if split.len > 0 and split[^1].len == 0:
    split.setLen(split.len - 1)
  for line in split:
    if lines >= ReadLines:
      break
    let kept = if line.len > LineChars: LineChars else: line.len
    result += kept + 1 + NumberBytes
    inc lines

proc readHint(event: JsonNode): int =
  if event{"hook_event_name"}.getStr("") != "PreToolUse" or
     event{"tool_name"}.getStr("") != "Read":
    return 0
  let session = event{"session_id"}.getStr("")
  if session.len == 0:
    return 0
  let input = event{"tool_input"}
  let hasOffset = input.hasKey("offset") and input{"offset"}.kind != JNull
  let hasLimit = input.hasKey("limit") and input{"limit"}.kind != JNull
  if hasOffset or hasLimit:
    return 0
  let path = input{"file_path"}.getStr("")
  if path.len == 0 or not fileExists(path):
    return 0
  let ext = path.splitFile.ext.toLowerAscii
  if ext.len > 0 and ext[1 .. ^1] in Rendered:
    return 0
  try:
    if getFileSize(path) > ReadCap:
      return 0
  except CatchableError:
    return 0
  var text = ""
  try: text = readFile(path) except CatchableError: return 0
  let bytes = estimate(text)
  if bytes <= Oversized:
    return 0
  let dir = readRecordDir()
  if dir.len == 0:
    return 0
  let record = dir / session.multiReplace(("-", "-"), (".", "_"), ("/", "_"))
  if fileExists(record):
    try:
      if path in readFile(record).splitLines:
        return 0
    except CatchableError: discard
  try:
    var file = open(record, fmAppend)
    file.writeLine(path)
    file.close()
  except CatchableError:
    return 0
  let reason = "A whole read of this file returns about " & $bytes &
    " bytes: read the range you need (`offset` and `limit`) or search it with `Grep`; " &
    "if you need it whole, repeat the same call, which then goes through."
  echo $(%*{"hookSpecificOutput": {
    "hookEventName": "PreToolUse",
    "permissionDecision": "deny",
    "permissionDecisionReason": reason,
  }})
  0

proc holderText(holder: Holder): string =
  "session " & holder.session & "\npid " & holder.pid & "\n"

proc readHolder(path: string): Holder =
  if not fileExists(path):
    return
  for line in readFile(path).splitLines:
    if line.startsWith("session "):
      result.session = line[8 .. ^1]
    elif line.startsWith("pid "):
      result.pid = line[4 .. ^1]

proc alive(pid: string): bool =
  try:
    posix.kill(Pid(parseInt(pid)), 0) == 0
  except CatchableError, ValueError:
    false

proc inProgress(cwd: string): bool =
  git(cwd, @["--no-optional-locks", "status", "--porcelain", "--untracked-files=normal"]).strip().len > 0

proc keeps(holder, me: Holder; cwd: string): bool =
  holder.session != me.session and holder.pid != me.pid and
    alive(holder.pid) and inProgress(cwd)

proc takeLock(path: string; me: Holder; cwd: string): bool =
  let before = readHolder(path)
  if before.session == me.session:
    return true
  if before.session.len > 0 and keeps(before, me, cwd):
    return false
  let tmp = path & ".tmp" & $getCurrentProcessId()
  try:
    writeFile(tmp, holderText(me))
    moveFile(tmp, path)
  except CatchableError:
    return false
  true

proc workspaceLock(args: seq[string]; event: JsonNode): int =
  let mode = if args.len > 0: args[0] else: ""
  if mode notin ["check", "start", "release"]:
    stderr.writeLine("usage: cardinal-nim run workspace-lock check | start | release")
    return 2
  let session = event{"session_id"}.getStr("")
  let cwd = event{"cwd"}.getStr("")
  let pid = getEnv("CLAUDE_PID")
  if session.len == 0 or cwd.len == 0 or pid.len == 0:
    return 0
  let path = gitPath(cwd, LockFile)
  if path.len == 0:
    return 0
  if mode == "release":
    if readHolder(path).session == session:
      removeFile(path)
    return 0
  let me = Holder(session: session, pid: pid)
  if takeLock(path, me, cwd):
    return 0
  let holder = readHolder(path)
  if mode == "start":
    echo "This worktree is busy: the session " & holder.session &
      " works in it and still runs. Your edits and commands will be refused here: " &
      "tell the user to open another worktree or to end that session."
    return 0
  stderr.writeLine("Refused: another live session (" & holder.session &
    ") works in this worktree; two sessions would share its files, index and review marker. " &
    "Stop and let the user open another worktree or end that session.")
  2

proc formatCode(event: JsonNode): int =
  let file = event{"tool_input", "file_path"}.getStr("")
  if file.len == 0 or not fileExists(file):
    return 0
  let dir = block:
    let parent = parentDir(file)
    if dirExists(parent): parent else: "."
  let root = findProjectRoot(dir)
  let project = if root.len > 0: readProject(root) else: ProjectConfig()
  let relative = if root.len > 0: relativePath(file, root) else: file.extractFilename
  if not actsOn(project.formatFiles, relative):
    return 0
  if project.formatDeclared:
    if project.formatCommand.len == 0:
      return 0
    discard execCapture("sh", @["-c", project.formatCommand & " \"$@\"", "sh", file],
                        (if root.len > 0: root else: dir), "")
  else:
    discard execCapture("cargo", @["fmt", "--all"], dir, "")
  0

proc proseWords(text: string): seq[string] =
  var prose = ""
  var fenced = false
  for line in text.splitLines:
    if line.strip.startsWith("```"):
      fenced = not fenced
      continue
    if fenced:
      continue
    let parts = line.split('`')
    var index = 0
    while index < parts.len:
      prose.add(parts[index])
      prose.add(" ")
      index += 2
    prose.add("\n")
  var word = ""
  for letter in prose:
    if letter.isAlphaAscii:
      word.add(letter.toLowerAscii)
    elif word.len > 0:
      result.add(word)
      word = ""
  if word.len > 0:
    result.add(word)

proc languageHint(event: JsonNode): int =
  if event{"hook_event_name"}.getStr("") != "Stop":
    return 0
  if event{"stop_hook_active"}.getBool(false):
    return 0
  let cwd = block:
    let fromEvent = event{"cwd"}.getStr("")
    if fromEvent.len > 0: fromEvent else: getEnv("CLAUDE_PROJECT_DIR", ".")
  let root = findProjectRoot(cwd)
  if root.len == 0:
    return 0
  let name = readProject(root).userLanguage.strip.toLowerAscii
  var own: seq[string]
  var other: seq[string]
  var display = ""
  case name
  of "french": own = @FrenchWords; other = @EnglishWords; display = "French"
  of "english": own = @EnglishWords; other = @FrenchWords; display = "English"
  else: return 0
  let text = event{"last_assistant_message"}.getStr("")
  if text.len == 0:
    return 0
  let words = proseWords(text)
  if words.len < MinWords:
    return 0
  var mine = 0
  var theirs = 0
  for word in words:
    if word in own: inc mine
    if word in other: inc theirs
  if theirs <= mine:
    return 0
  stderr.writeLine("Your last reply is not in " & display &
    ", the user_language of .claude/cardinal.toml (or of .claude/cardinal.local.toml, which overrides it): " &
    "give it again in " & display & ".")
  2

## A neutral event from the adapter: {action, tool, input, session, cwd,
## stop_active, last_message, success}. Legacy Claude-shaped events pass through.
proc normalize(event: JsonNode): JsonNode =
  if not event.hasKey("action"):
    return event
  result = newJObject()
  if event.hasKey("cwd"): result["cwd"] = event{"cwd"}
  if event.hasKey("session"): result["session_id"] = event{"session"}
  if event.hasKey("stop_active"): result["stop_hook_active"] = event{"stop_active"}
  if event.hasKey("last_message"): result["last_assistant_message"] = event{"last_message"}
  if event.hasKey("input"): result["tool_input"] = event{"input"}
  if event.hasKey("success"):
    result["tool_response"] = %*{"success": event{"success"}.getBool(false)}
  case event{"action"}.getStr("")
  of "shell":
    result["hook_event_name"] = %"PreToolUse"
    result["tool_name"] = %"Bash"
  of "read":
    result["hook_event_name"] = %"PreToolUse"
    result["tool_name"] = %"Read"
  of "skill":
    result["hook_event_name"] = %"PostToolUse"
    result["tool_name"] = %"Skill"
  of "format":
    result["hook_event_name"] = %"PostToolUse"
    result["tool_name"] = %"Edit"
  of "idle":
    result["hook_event_name"] = %"Stop"
  else: discard

proc runTool*(root, tool: string; args: seq[string]; input: string): int =
  var event: JsonNode
  try:
    event = parseJson(input)
  except CatchableError, ValueError:
    event = newJObject()
  event = normalize(event)
  let cwd = block:
    let fromEvent = event{"cwd"}.getStr("")
    if fromEvent.len > 0: fromEvent else: root
  case tool
  of "review-gate":
    let mode = if args.len > 0: args[0] else: ""
    case mode
    of "check": reviewCheck(event)
    of "record": reviewRecord(event, cwd)
    of "native": reviewNative(cwd)
    else: 3
  of "lint-gate": lintGate(event)
  of "read-hint": readHint(event)
  of "workspace-lock": workspaceLock(args, event)
  of "format-code": formatCode(event)
  of "language-hint": languageHint(event)
  of "session-metrics": runSessionMetrics(args)
  of "commit-check": commitCheck(root, input)
  of "check-commits": checkCommits(root, args)
  of "check-issue": checkIssue(root, args)
  of "next-issue": nextIssue(root, args)
  of "close-issue": closeIssue(root, args)
  of "issue-claim": issueClaim(root, args)
  of "review-target": reviewTarget(root, args)
  of "deliverable-size": deliverableSize(root, args)
  of "worktree": worktree(root, args)
  of "release": release(root, args)
  of "check-specs": checkSpecs(root, args)
  of "spec-coverage": specCoverage(root, args)
  of "fixed-context": fixedContext(root, args)
  of "audit-sources": auditSources(root, args)
  else:
    const Retired = ["cardinal-changes", "cardinal-report", "delivered-quality",
                     "rule-breaches", "upgrade"]
    if tool in Retired:
      stderr.writeLine("cardinal: '" & tool & "' is retired; use `cardinal run session-metrics`")
    else:
      stderr.writeLine("cardinal: unknown tool " & tool)
    3
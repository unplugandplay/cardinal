## The workflow helpers: commit rules, issue queue, review target, size,
## worktrees, specs and release.

import std/[algorithm, os, strutils, times]

import config, shell

const Types = ["feat", "fix", "refactor", "docs", "test", "perf", "build", "ci", "chore"]

type
  Issue = object
    path, stem, priority, why: string
    depends: seq[string]

proc checkMessage(message: string; scopes: seq[string]): seq[string] =
  let lines = message.strip.splitLines
  if lines.len == 0 or lines[0].strip.len == 0:
    return @["empty message"]
  let subject = lines[0].strip
  if subject.len > 72:
    result.add("subject longer than 72 characters")
  let colon = subject.find(": ")
  if colon < 0:
    result.add("subject is not `<type>(<scope>): <description>`")
  else:
    let head = subject[0 ..< colon]
    let description = subject[colon + 2 .. ^1].strip
    var kind = head
    var scope = ""
    let open = head.find('(')
    if open >= 0:
      kind = head[0 ..< open]
      if head.endsWith(")"):
        scope = head[open + 1 .. ^2]
      else:
        result.add("scope is not closed")
    if kind notin Types:
      result.add("type `" & kind & "` is not conventional")
    let allowed = @["docs", "cardinal"] & scopes
    if scope.len > 0 and scope notin allowed:
      result.add("scope `" & scope & "` is not in commit_scopes")
    if description.len == 0:
      result.add("empty description")
    else:
      if description[0].isUpperAscii:
        result.add("description starts with a capital letter")
      if description.endsWith("."):
        result.add("description ends with a period")
  for line in lines:
    let lower = line.toLowerAscii
    if lower.contains("co-authored-by") or lower.contains("noreply@anthropic.com") or
       (lower.contains("generated with") and lower.contains("claude")):
      result.add("AI attribution in the message")

proc commitCheck*(root: string; input: string): int =
  let scopes = readProject(root).commitScopes
  let problems = checkMessage(input, scopes)
  for problem in problems:
    stderr.writeLine(problem)
  if problems.len > 0: 1 else: 0

proc checkCommits*(root: string; args: seq[string]): int =
  if args.len < 2:
    stderr.writeLine("usage: cardinal run check-commits <base> <head>")
    return 3
  let scopes = readProject(root).commitScopes
  let log = git(root, @["log", "--no-merges", "--format=%H%x1f%B%x1e",
                        args[0] & ".." & args[1]])
  var failed = false
  for record in log.split('\x1e'):
    let text = record.strip
    if text.len == 0:
      continue
    let cut = text.find('\x1f')
    if cut < 0:
      continue
    let hash = text[0 ..< cut]
    let message = text[cut + 1 .. ^1]
    for problem in checkMessage(message, scopes):
      stderr.writeLine(hash[0 ..< min(10, hash.len)] & ": " & problem)
      failed = true
  if failed: 1 else: 0

proc parseIssue(path: string): Issue =
  result.path = path
  result.stem = path.extractFilename.changeFileExt("")
  var front = false
  var closed = false
  for raw in lines(path):
    let line = raw.strip
    if line == "---":
      if front: closed = true
      front = not front
      continue
    if front and not closed:
      let eq = line.find(':')
      if eq < 0: continue
      let key = line[0 ..< eq].strip
      var value = line[eq + 1 .. ^1].strip()
      if value.len >= 2 and (value[0] == '\'' or value[0] == '"') and value[^1] == value[0]:
        value = value[1 .. ^2]
      case key
      of "priority": result.priority = value
      of "why": result.why = value
      of "depends_on":
        let open = value.find('[')
        let close = value.rfind(']')
        if open >= 0 and close > open:
          for part in value[open + 1 ..< close].split(','):
            let item = part.strip().strip(chars = {'\'', '"'})
            if item.len > 0:
              result.depends.add(item)

proc requiredHeadings(path: string): seq[string] =
  for raw in lines(path):
    let line = raw.strip
    if line.startsWith("# ") and line.count('#') == 1:
      result.add(line[2 .. ^1].strip)

proc checkIssueFile(path: string): seq[string] =
  if not fileExists(path):
    return @["no such file: " & path]
  let stem = path.extractFilename.changeFileExt("")
  for letter in stem:
    if not (letter.isLowerAscii or letter.isDigit or letter == '_'):
      result.add("file name is not snake_case")
      break
  let issue = parseIssue(path)
  if issue.priority notin ["high", "medium", "low"]:
    result.add("priority is not high, medium or low")
  if issue.why.len == 0:
    result.add("no why in the front matter")
  let headings = requiredHeadings(path)
  if headings.len == 0:
    result.add("no title")
  for needed in ["Goal", "Approach", "Resources"]:
    if needed notin headings:
      result.add("no `# " & needed & "` section")
  for raw in lines(path):
    if raw.contains("/Users/") or raw.contains("/home/") or raw.contains("~/"):
      result.add("absolute path in the issue")
      break

proc checkIssue*(root: string; args: seq[string]): int =
  if args.len == 0:
    stderr.writeLine("usage: cardinal run check-issue <file>")
    return 3
  var problems: seq[string]
  for arg in args:
    problems.add(checkIssueFile(if arg.isAbsolute: arg else: root / arg))
  for problem in problems:
    stderr.writeLine(problem)
  if problems.len > 0: 1 else: 0

proc issueStems(root: string): seq[string] =
  let dir = root / "issues"
  if not dirExists(dir):
    return
  for kind, path in walkDir(dir):
    if kind == pcFile and path.endsWith(".md"):
      result.add(path.extractFilename.changeFileExt(""))
  result.sort()

proc doneStems(root: string): seq[string] =
  let dir = root / "issues" / "done"
  if not dirExists(dir):
    return
  for kind, path in walkDir(dir):
    if kind == pcFile and path.endsWith(".md"):
      result.add(path.extractFilename.changeFileExt(""))

proc takenStems(root: string): seq[string] =
  let claims = git(root, @["for-each-ref", "--format=%(refname:short)",
                           "refs/cardinal/claims"])
  for line in claims.splitLines:
    let name = line.strip.split('/')[^1]
    if name.len > 0: result.add(name)
  let branches = git(root, @["branch", "--format=%(refname:short)"])
  for line in branches.splitLines:
    let name = line.strip
    if name.startsWith("issue/"):
      result.add(name[6 .. ^1])

proc priorityRank(priority: string): int =
  case priority
  of "high": 0
  of "medium": 1
  of "low": 2
  else: 3

proc nextIssue*(root: string; args: seq[string]): int =
  let done = doneStems(root)
  let taken = takenStems(root)
  var ready: seq[(int, string, Issue)]
  var blocked: seq[string]
  for stem in issueStems(root):
    let issue = parseIssue(root / "issues" / (stem & ".md"))
    var missing: seq[string]
    for dep in issue.depends:
      if dep notin done:
        missing.add(dep)
    if missing.len > 0:
      blocked.add(stem & " (needs " & missing.join(", ") & ")")
    elif stem notin taken:
      ready.add((priorityRank(issue.priority), stem, issue))
  ready.sort(proc (a, b: (int, string, Issue)): int = cmp(a[0], b[0]))
  if ready.len == 0:
    echo "no ready issue"
  else:
    let (_, stem, issue) = ready[0]
    echo "next: issues/" & stem & ".md (" & issue.priority & ")"
  for line in blocked:
    echo "blocked: " & line
  0

proc closeIssue*(root: string; args: seq[string]): int =
  if args.len == 0:
    stderr.writeLine("usage: cardinal run close-issue <file>")
    return 3
  let given = if args[0].isAbsolute: args[0] else: root / args[0]
  let path = if fileExists(given): given
             elif fileExists(root / "issues" / (args[0] & ".md")): root / "issues" / (args[0] & ".md")
             else: given
  if not fileExists(path):
    stderr.writeLine("no such issue: " & args[0])
    return 1
  let stem = path.extractFilename.changeFileExt("")
  let done = root / "issues" / "done"
  createDir(done)
  moveFile(path, done / (stem & ".md"))
  let old = "issues/" & stem & ".md"
  let fresh = "issues/done/" & stem & ".md"
  for file in walkDirRec(root, yieldFilter = {pcFile}):
    if not file.endsWith(".md") or file.contains("/.git/") or file.contains("/harness/"):
      continue
    if file.contains("/issues/done/"):
      continue
    try:
      let text = readFile(file)
      if text.contains(old):
        writeFile(file, text.replace(old, fresh))
    except CatchableError:
      discard
  echo "closed: issues/done/" & stem & ".md"
  discard nextIssue(root, @[])
  0

proc issueClaim*(root: string; args: seq[string]): int =
  if args.len == 0:
    stderr.writeLine("usage: cardinal run issue-claim <file>")
    return 3
  let stem = args[0].extractFilename.changeFileExt("")
  let session = getEnv("CLAUDE_CODE_SESSION_ID", getEnv("CARDINAL_SESSION", ""))
  if session.len == 0:
    stderr.writeLine("no session id (CLAUDE_CODE_SESSION_ID or CARDINAL_SESSION)")
    return 1
  discard execCapture("git", @["update-ref", "refs/cardinal/claims/" & stem, session], root, "")
  echo "claimed: " & stem
  0

proc reviewTarget*(root: string; args: seq[string]): int =
  let base = if args.len > 0: args[0] else: "HEAD"
  var names: seq[string]
  for line in git(root, @["diff", "--name-only", base]).splitLines:
    if line.strip.len > 0: names.add(line.strip)
  for line in git(root, @["ls-files", "--others", "--exclude-standard"]).splitLines:
    if line.strip.len > 0: names.add(line.strip)
  var managed: seq[string]
  let lock = root / ".claude" / "cardinal.lock"
  if fileExists(lock):
    for line in lines(lock):
      let sep = line.find("  ")
      if sep > 0: managed.add(line[sep + 2 .. ^1])
  names.sort()
  for name in names:
    if name.startsWith("issues/") or name in managed:
      continue
    echo name
  0

proc deliverableSize*(root: string; args: seq[string]): int =
  let rev = if args.len > 0: args[0] else: "HEAD"
  var linesChanged = 0
  for line in git(root, @["diff", "--numstat", rev]).splitLines:
    let parts = line.split('\t')
    if parts.len < 2: continue
    try:
      linesChanged += parseInt(parts[0]) + parseInt(parts[1])
    except ValueError:
      discard
  let untracked = git(root, @["ls-files", "--others", "--exclude-standard"])
  for name in untracked.splitLines:
    let path = root / name.strip
    if name.strip.len > 0 and fileExists(path):
      try: linesChanged += readFile(path).countLines
      except CatchableError: discard
  let limit = readProject(root).deliverableLimit
  echo "changed lines: " & $linesChanged & " (limit " & $limit & ")"
  if linesChanged > limit:
    echo "over the deliverable limit"
  0

proc worktree*(root: string; args: seq[string]): int =
  if args.len < 2:
    stderr.writeLine("usage: cardinal run worktree open <name> [--branch <b>] [--path <p>] | close <name|path> [--force]")
    return 3
  let mode = args[0]
  if mode == "open":
    var branch = args[1]
    var path = parentDir(root) / (root.extractFilename & "-" & args[1])
    var index = 2
    while index < args.len:
      case args[index]
      of "--branch":
        inc index
        if index < args.len: branch = args[index]
      of "--path":
        inc index
        if index < args.len: path = args[index]
      else: discard
      inc index
    let result = execCapture("git", @["worktree", "add", "-b", branch, path], root, "")
    if result.code != 0:
      stderr.writeLine(result.output.strip)
      return 1
    echo path
    return 0
  if mode == "close":
    var path = args[1]
    if not path.contains("/"):
      path = parentDir(root) / (root.extractFilename & "-" & path)
    var flags: seq[string]
    if "--force" in args[2 .. ^1]: flags.add("--force")
    let result = execCapture("git", @["worktree", "remove"] & flags & @[path], root, "")
    if result.code != 0:
      stderr.writeLine(result.output.strip)
      return 1
    return 0
  3

proc isSemver(version: string): bool =
  let parts = version.strip(chars = {'v'}).split('.')
  if parts.len != 3:
    return false
  for part in parts:
    if part.len == 0:
      return false
    for letter in part:
      if not letter.isDigit:
        return false
  true

proc bump(version: string; level: int): string =
  var parts = version.strip(chars = {'v'}).split('.')
  while parts.len < 3: parts.add("0")
  var major = parseInt(parts[0])
  var minor = parseInt(parts[1])
  var patch = parseInt(parts[2])
  if level == 0: inc major; minor = 0; patch = 0
  elif level == 1: inc minor; patch = 0
  else: inc patch
  $major & "." & $minor & "." & $patch

proc release*(root: string; args: seq[string]): int =
  let tagResult = execCapture("git", @["describe", "--tags", "--abbrev=0"], root, "")
  let tag = if tagResult.code == 0: tagResult.output.strip() else: ""
  let range = if tag.len > 0: tag & "..HEAD" else: "HEAD"
  let subjects = git(root, @["log", "--reverse", "--format=%s", range]).splitLines
  var level = 2
  for subject in subjects:
    if subject.contains("!:") or subject.startsWith("feat"):
      level = min(level, (if subject.contains("!:"): 0 else: 1))
  let proposed = if tag.len > 0 and isSemver(tag): bump(tag, level) else: "0.1.0"
  var version = ""
  var doTag = false
  for arg in args:
    if arg == "--tag": doTag = true
    elif not arg.startsWith("-"): version = arg
  if version.len == 0:
    echo proposed
    return 0
  var section = "## v" & version & " — " & $now().utc.format("yyyy-MM-dd") & "\n"
  var added, fixed, changed: seq[string]
  for subject in subjects:
    if subject.len == 0: continue
    if subject.startsWith("feat"): added.add("- " & subject)
    elif subject.startsWith("fix"): fixed.add("- " & subject)
    else: changed.add("- " & subject)
  for (title, list) in [("Added", added), ("Fixed", fixed), ("Changed", changed)]:
    if list.len == 0: continue
    section.add("\n### " & title & "\n\n" & list.join("\n") & "\n")
  let changelog = root / "CHANGELOG.md"
  var text = if fileExists(changelog): readFile(changelog) else: "# Changelog\n"
  var lines = text.splitLines
  var at = 0
  for index, line in lines:
    if line.startsWith("## "):
      at = index
      break
  let head = if at > 0: lines[0 ..< at].join("\n") & "\n" else: text
  let tail = if at > 0: lines[at .. ^1].join("\n") else: ""
  writeFile(changelog, head & "\n" & section & (if tail.len > 0: "\n" & tail & "\n" else: ""))
  echo "wrote CHANGELOG.md section v" & version
  if doTag:
    let result = execCapture("git", @["tag", "v" & version], root, "")
    if result.code != 0:
      stderr.writeLine(result.output.strip)
      return 1
    echo "tagged v" & version & " (not pushed)"
  0

proc checkSpecs*(root: string; args: seq[string]): int =
  let dir = root / "specs"
  if not dirExists(dir):
    return 0
  var failed = false
  for kind, path in walkDir(dir):
    if kind != pcFile or not path.endsWith(".md"):
      continue
    let name = path.extractFilename
    if name in ["README.md", "TEMPLATE.md"]:
      continue
    var text = ""
    try: text = readFile(path) except CatchableError: continue
    for needed in ["## Intent", "## Requirements", "## Scenarios", "## Non-goals"]:
      if not text.contains(needed):
        stderr.writeLine(name & ": missing `" & needed & "`")
        failed = true
    var ids: seq[string]
    for line in text.splitLines:
      let marker = line.find("**")
      if marker < 0 or not line.strip.startsWith("- **"): continue
      let close = line.find("**", marker + 2)
      if close < 0: continue
      let id = line[marker + 2 ..< close]
      if id in ids:
        stderr.writeLine(name & ": duplicate requirement " & id)
        failed = true
      ids.add(id)
  if failed: 1 else: 0

proc specCoverage*(root: string; args: seq[string]): int =
  var missing = 0
  for path in walkDirRec(root):
    if path.contains("/.git/") or path.contains("/harness/") or path.contains("/nim/bin/"):
      continue
    let name = path.extractFilename
    let isTest = name.endsWith("_test.nim") or name.endsWith("_test.rs") or
                 path.contains("/tests/")
    if not isTest:
      continue
    var text = ""
    try: text = readFile(path) except CatchableError: continue
    if not text.contains("spec:"):
      echo path.relativePath(root)
      inc missing
  if missing == 0:
    echo "every test file cites a spec"
  0

proc fixedContext*(root: string; args: seq[string]): int =
  var total = 0
  for name in ["AGENTS.md", "CLAUDE.md"]:
    let path = root / name
    if fileExists(path):
      try: total += readFile(path).len except CatchableError: discard
  let skills = root / ".claude" / "skills"
  if dirExists(skills):
    for path in walkDirRec(skills):
      if path.endsWith("SKILL.md"):
        try: total += readFile(path).len except CatchableError: discard
  echo "fixed context: " & $total & " bytes"
  0

proc auditSources*(root: string; args: seq[string]): int =
  var total = 0
  for path in walkDirRec(root / ".claude", yieldFilter = {pcFile}):
    if path.endsWith(".md") or path.endsWith(".toml") or path.endsWith(".json"):
      try: total += readFile(path).len except CatchableError: discard
  echo "instruction sources: " & $total & " bytes"
  0
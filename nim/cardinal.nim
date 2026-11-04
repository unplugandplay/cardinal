## Cardinal in Nim — phase 1: sync and check.
##
## The contract is the one the Rust binary established: `source/files/` is
## managed, `source/seed/` is copied once, `.claude/cardinal.lock` holds the
## SHA-256 of each managed file and the managed settings entries (preserved
## verbatim here until the entries port lands).
##
## Build: nim c -d:release -o:nim/bin/cardinal-nim nim/cardinal.nim

import std/[algorithm, os, streams, strutils, tables]

import config, entries, gates, sha256

const
  ConfigPath = ".claude/cardinal.toml"
  LockPath = ".claude/cardinal.lock"
  LockFormat = 8
  Usage = """cardinal-nim — the Nim Cardinal harness (phase 1)

usage:
  cardinal-nim check            fail when a managed copy drifted
  cardinal-nim sync [--dry-run] [--force]
  cardinal-nim run <tool> [args]  a gate, reading the event on stdin
  cardinal-nim --version"""

type
  Lock = object
    format: int
    files: OrderedTable[string, string] # project path -> hash, `+x` when executable
    entries: seq[string]                # preserved `entry` lines (settings)

  Outcome = enum
    oAdded, oUpdated, oRemoved, oSeeded, oUnchanged, oConflict, oKept

proc die(message: string) =
  stderr.writeLine("cardinal-nim: " & message)
  quit(1)

proc slash(path: string): string = path.replace('\\', '/')

proc readLock(root: string): Lock =
  let path = root / LockPath
  if not fileExists(path):
    return
  for raw in lines(path):
    let line = raw.strip()
    if line.len == 0 or line.startsWith("#"):
      continue
    if line.startsWith("format "):
      try:
        result.format = parseInt(line[7 .. ^1].strip())
      except ValueError:
        die("unreadable format line in " & LockPath)
      continue
    if line.startsWith("entry  "):
      result.entries.add(line[7 .. ^1])
      continue
    let sep = line.find("  ")
    if sep < 0:
      continue
    result.files[line[sep + 2 .. ^1]] = line[0 ..< sep]

proc baseHash(hash: string): string =
  if hash.endsWith("+x"): hash[0 .. ^3] else: hash

proc hashOf(path: string): string =
  sha256Hex(readFile(path))

proc recordHash(path: string): string =
  let hash = hashOf(path)
  when defined(posix):
    if fpUserExec in getFilePermissions(path):
      return hash & "+x"
  hash

proc sortedFiles(files: OrderedTable[string, string]): seq[(string, string)] =
  for path, hash in files:
    result.add((path, hash))
  result.sort(proc (a, b: (string, string)): int = cmp(a[0], b[0]))

proc writeLock(root: string, files: OrderedTable[string, string], entries: seq[string]) =
  var text = "# Written by `cardinal sync`: hashes of the managed files, then the managed settings entries. Do not edit.\n"
  text.add("format " & $LockFormat & "\n")
  for (path, hash) in sortedFiles(files):
    text.add(hash & "  " & path & "\n")
  for entry in entries:
    text.add("entry  " & entry & "\n")
  writeFile(root / LockPath, text)

proc sourceEntries(source, dir: string): seq[string] =
  let base = source / dir
  if not dirExists(base):
    return
  for path in walkDirRec(base, yieldFilter = {pcFile}, followFilter = {pcDir}):
    result.add(slash(relativePath(path, base)))

proc copyManaged(source, root, rel: string) =
  let src = source / "files" / rel
  let to = root / rel
  createDir(parentDir(to))
  copyFileWithPermissions(src, to)

proc cmdCheck(root: string) =
  let config = readSource(root)
  if config.source.len == 0:
    die("no source in " & ConfigPath)
  let lock = readLock(root)
  if lock.format == 0:
    die("no " & LockPath & ": run cardinal sync")
  var drifted = 0
  for path, expected in lock.files:
    let file = root / path
    if not fileExists(file):
      stderr.writeLine("missing: " & path)
      inc drifted
      continue
    if hashOf(file) != baseHash(expected):
      stderr.writeLine("drifted: " & path)
      inc drifted
  let settings = readSettings(root / SettingsPath)
  for line in lock.entries:
    var entry: Entry
    try:
      entry = fromLock(line)
    except CatchableError, ValueError:
      continue
    if settings.isNil or not holds(settings, entry):
      stderr.writeLine("missing entry: " & label(entry))
      inc drifted
  if not hookInstalled(root):
    stderr.writeLine("missing: native review gate")
    inc drifted
  if drifted > 0:
    stderr.writeLine("cardinal-nim: " & $drifted & " managed file(s) or entries drifted; run cardinal sync or fix the source")
    quit(1)

proc cmdSync(root: string, dryRun, force: bool) =
  let config = readSource(root)
  if config.source.len == 0:
    die("no source in " & ConfigPath)
  let source = (if config.source.isAbsolute: config.source else: root / config.source).absolutePath
  if not dirExists(source / "files"):
    die("no files/ in source " & source)
  let previous = readLock(root)
  var next: OrderedTable[string, string]
  var changed = 0

  let managed = sourceEntries(source, "files")
  for rel in managed:
    let src = source / "files" / rel
    let to = root / rel
    let sourceHash = recordHash(src)
    let old = previous.files.getOrDefault(rel, "")
    var conflict = false
    if not fileExists(to):
      if not dryRun: copyManaged(source, root, rel)
      echo "added: ", rel
      inc changed
    else:
      let current = recordHash(to)
      if current == sourceHash:
        discard
      elif old.len == 0 or baseHash(current) == baseHash(old):
        if not dryRun: copyManaged(source, root, rel)
        echo "updated: ", rel
        inc changed
      elif force:
        if not dryRun: copyManaged(source, root, rel)
        echo "updated: ", rel
        inc changed
      else:
        echo "conflict: ", rel, " (modified locally, fix it in the source or use --force)"
        inc changed
        conflict = true
    if conflict:
      # Keep the hash last written, so the conflict is seen again next time.
      next[rel] = if old.len > 0: old else: recordHash(to)
    elif fileExists(to):
      next[rel] = recordHash(to)
    elif dryRun:
      next[rel] = sourceHash

  # Managed files dropped from the source: removed when untouched, kept otherwise.
  for rel, old in previous.files:
    if managed.contains(rel):
      continue
    let to = root / rel
    if not fileExists(to):
      continue
    if baseHash(recordHash(to)) == baseHash(old) or force:
      if not dryRun: removeFile(to)
      echo "removed: ", rel
      inc changed
    else:
      echo "kept: ", rel, " (dropped from the source but modified locally)"
      inc changed

  # Seeds: copied once, then owned by the project.
  for rel in sourceEntries(source, "seed"):
    let to = root / rel
    if fileExists(to):
      continue
    let src = source / "seed" / rel
    if not dryRun:
      createDir(parentDir(to))
      copyFileWithPermissions(src, to)
    echo "seeded: ", rel
    inc changed

  let wanted = ofSource(source)
  var recorded: seq[Entry]
  for line in previous.entries:
    try:
      recorded.add(fromLock(line))
    except CatchableError, ValueError:
      discard
  let settingsFile = root / SettingsPath
  var settings = readSettings(settingsFile)
  var lockEntries = previous.entries
  if not settings.isNil:
    var outcomes: seq[string]
    let mergeChanged = merge(settings, wanted, recorded, force, outcomes)
    for line in outcomes:
      echo line
      inc changed
    if mergeChanged and not dryRun:
      writeSettings(settingsFile, settings)
    lockEntries = @[]
    for entry in wanted:
      lockEntries.add(toLock(entry))
  elif fileExists(settingsFile):
    echo "warning: " & SettingsPath & " is not valid JSON; its managed entries are not synced"

  let gate = installReviewHook(root, dryRun)
  if gate.len > 0:
    echo gate
    inc changed

  if not dryRun:
    writeLock(root, next, lockEntries)
  if changed == 0:
    echo "up to date"
  elif dryRun:
    echo "(dry run: nothing written)"

when isMainModule:
  let args = commandLineParams()
  if args.len == 0 or args[0] in ["-h", "--help", "help"]:
    echo Usage
    quit(0)
  if args[0] in ["-v", "--version", "version"]:
    echo "cardinal-nim 0.1.0 (phase 1)"
    quit(0)
  let root = getCurrentDir()
  case args[0]
  of "check":
    cmdCheck(root)
  of "run":
    if args.len < 2:
      die("usage: cardinal-nim run <tool> [args]")
    let input = readAll(stdin)
    quit(runTool(root, args[1], (if args.len > 2: args[2 .. ^1] else: @[]), input))
  of "sync":
    var dryRun = false
    var force = false
    if args.len > 1:
      for arg in args[1 .. ^1]:
        case arg
        of "--dry-run": dryRun = true
        of "--force": force = true
        else: die("unknown option " & arg)
    cmdSync(root, dryRun, force)
  else:
    die("unknown command " & args[0] & "\n" & Usage)
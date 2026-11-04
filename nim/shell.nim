## Thin process helpers shared by the sync and the gates.

import std/[os, osproc, streams, strutils]

proc execCapture*(command: string; args: seq[string]; cwd: string;
                  input: string): tuple[output: string, code: int] =
  try:
    var process = startProcess(command, workingDir = cwd, args = args,
                               options = {poUsePath, poStdErrToStdOut})
    if input.len > 0:
      process.inputStream.write(input)
      process.inputStream.flush()
    process.inputStream.close()
    result.output = process.outputStream.readAll()
    result.code = process.waitForExit()
    process.close()
  except CatchableError:
    result = ("", 127)

proc git*(cwd: string; args: seq[string]; input = ""): string =
  execCapture("git", args, cwd, input).output

proc gitPath*(cwd, name: string): string =
  let path = git(cwd, @["rev-parse", "--git-path", name]).strip()
  if path.len == 0:
    return ""
  if path.isAbsolute: path else: cwd / path

proc topOf*(cwd: string): string =
  let top = git(cwd, @["rev-parse", "--show-toplevel"]).strip()
  if top.len == 0: cwd else: top
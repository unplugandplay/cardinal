## Minimal reader for `.claude/cardinal.toml` and `cardinal.local.toml`.
## Only the keys the harness reads are parsed: source/rev and the `[project]` values.

import std/[os, strutils, tables]

const
  ConfigPath* = ".claude/cardinal.toml"
  PersonalConfigPath* = ".claude/cardinal.local.toml"

type
  SourceConfig* = object
    source*: string
    rev*: string

  ProjectConfig* = object
    commitScopes*: seq[string]
    deliverableLimit*: int
    issueLimit*: int
    lintCommand*: string
    lintDeclared*: bool
    lintFiles*: seq[string]
    formatCommand*: string
    formatDeclared*: bool
    formatFiles*: seq[string]
    userLanguage*: string

proc stripComment(value: string): string =
  var quoted = false
  var i = 0
  while i < value.len:
    case value[i]
    of '"': quoted = not quoted
    of '#':
      if not quoted and i > 0 and value[i - 1] == ' ':
        return value[0 ..< i].strip()
    else: discard
    inc i
  value.strip()

proc parseTable(path: string): Table[string, Table[string, string]] =
  result = initTable[string, Table[string, string]]()
  if not fileExists(path):
    return
  var section = ""
  for raw in lines(path):
    let line = raw.strip()
    if line.len == 0 or line.startsWith("#"):
      continue
    if line.startsWith("[") and line.endsWith("]"):
      section = line[1 .. ^2].strip()
      if not result.hasKey(section):
        result[section] = initTable[string, string]()
      continue
    let eq = line.find('=')
    if eq < 0:
      continue
    let key = line[0 ..< eq].strip()
    let value = stripComment(line[eq + 1 .. ^1])
    if not result.hasKey(section):
      result[section] = initTable[string, string]()
    result[section][key] = value

proc unquote(value: string): string =
  result = value.strip()
  if result.len >= 2 and result[0] == '"' and result[^1] == '"':
    result = result[1 .. ^2]

proc arrayOf(value: string): seq[string] =
  let text = value.strip()
  if text.len < 2 or text[0] != '[' or text[^1] != ']':
    return
  for part in text[1 .. ^2].split(','):
    let item = unquote(part.strip())
    if item.len > 0:
      result.add(item)

proc readSource*(root: string): SourceConfig =
  let table = parseTable(root / ConfigPath)
  if table.hasKey("cardinal"):
    result.source = unquote(table["cardinal"].getOrDefault("source", ""))
    result.rev = unquote(table["cardinal"].getOrDefault("rev", ""))

proc readProject*(root: string): ProjectConfig =
  result.deliverableLimit = 1000
  result.issueLimit = 3000
  let shared = parseTable(root / ConfigPath)
  let personal = parseTable(root / PersonalConfigPath)
  if shared.hasKey("project"):
    let table = shared["project"]
    if table.hasKey("lint_command"):
      result.lintDeclared = true
      result.lintCommand = unquote(table["lint_command"])
    if table.hasKey("lint_files"):
      result.lintFiles = arrayOf(table["lint_files"])
    if table.hasKey("format_command"):
      result.formatDeclared = true
      result.formatCommand = unquote(table["format_command"])
    if table.hasKey("format_files"):
      result.formatFiles = arrayOf(table["format_files"])
    result.userLanguage = unquote(table.getOrDefault("user_language", ""))
    result.commitScopes = arrayOf(table.getOrDefault("commit_scopes", "[]"))
    try:
      result.deliverableLimit = parseInt(unquote(table.getOrDefault("deliverable_size_limit", "1000")))
    except ValueError:
      discard
    try:
      result.issueLimit = parseInt(unquote(table.getOrDefault("issue_size_limit", "3000")))
    except ValueError:
      discard
  if personal.hasKey("project"):
    let table = personal["project"]
    if table.hasKey("user_language"):
      result.userLanguage = unquote(table["user_language"])

proc findProjectRoot*(start: string): string =
  var dir = start.absolutePath
  while true:
    if fileExists(dir / ConfigPath):
      return dir
    let parent = parentDir(dir)
    if parent.len == 0 or parent == dir:
      return ""
    dir = parent
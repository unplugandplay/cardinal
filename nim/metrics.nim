## `session-metrics`: one TSV row per session from the mirrored transcripts.
## The header matches the original reader; fields with no neutral source (versions,
## advisors, validations) stay empty.

import std/[algorithm, json, os, sets, strutils, tables, times]

import shell

const Header = "session\tproject\tissues\trevision\tmodels\tversions\tstart\tduration_s\tturns\ttool_calls\tinput_tokens\toutput_tokens\tthinking_tokens\tcache_read_tokens\tcache_creation_tokens\tmain_context_peak\tadvisor_models\tadvisor_calls\tadvisor_input_tokens\tadvisor_output_tokens\thook_refusals\tskills\tvalidations\tother_messages\tunreadable_lines\tunknown_lines\tmissing_fields"

type
  Row = object
    session: string
    project: string
    models: seq[string]
    skills: seq[string]
    start: string
    started: DateTime
    ended: DateTime
    hasStart: bool
    turns: int
    toolCalls: int
    inputTokens, outputTokens, thinkingTokens: int64
    cacheRead, cacheWrite, peak: int64
    refusals, other, missing, unreadable, unknown: int

proc parseTime(text: string): DateTime =
  for format in ["yyyy-MM-dd'T'HH:mm:ss'.'fff'Z'", "yyyy-MM-dd'T'HH:mm:ss'Z'"]:
    try:
      return parse(text, format, utc())
    except CatchableError, ValueError:
      discard
  raise newException(ValueError, "bad timestamp: " & text)

proc readSession(path: string): Row =
  result.session = path.extractFilename.changeFileExt("")
  result.started = fromUnix(0).utc
  result.ended = fromUnix(0).utc
  var seen = initHashSet[string]()
  for line in lines(path):
    if line.strip.len == 0:
      continue
    var node: JsonNode
    try:
      node = parseJson(line)
    except CatchableError, ValueError:
      inc result.unreadable
      continue
    let kind = node{"type"}.getStr("")
    if kind notin ["user", "assistant"]:
      inc result.unknown
      continue
    let stamp = node{"timestamp"}.getStr("")
    var at: DateTime
    try:
      at = parseTime(stamp)
    except ValueError:
      inc result.missing
      continue
    if not result.hasStart or at < result.started:
      result.started = at
      result.start = stamp
      result.hasStart = true
    if at > result.ended:
      result.ended = at
    if result.project.len == 0:
      let cwd = node{"cwd"}.getStr("")
      if cwd.len > 0:
        result.project = cwd.splitPath.tail
    if kind == "user":
      let text = block:
        let content = node{"message", "content"}
        if content.kind == JString: content.getStr("")
        elif content.kind == JArray:
          var parts: seq[string]
          for part in content:
            if part{"type"}.getStr("") == "text": parts.add(part{"text"}.getStr(""))
          parts.join("\n")
        else: ""
      if text.contains("reports problems") or text.contains("Your last reply is not in"):
        inc result.refusals
      continue
    # assistant
    let message = node{"message"}
    let usage = message{"usage"}
    let id = message{"id"}.getStr("")
    if id.len == 0 or usage.kind != JObject:
      inc result.missing
      continue
    if id in seen:
      continue
    seen.incl(id)
    inc result.turns
    let model = message{"model"}.getStr("")
    if model.len > 0 and model notin result.models:
      result.models.add(model)
    let taken = usage{"input_tokens"}.getInt + usage{"cache_read_input_tokens"}.getInt +
                usage{"cache_creation_input_tokens"}.getInt
    if taken > result.peak:
      result.peak = taken
    result.inputTokens += usage{"input_tokens"}.getInt
    result.outputTokens += usage{"output_tokens"}.getInt
    result.thinkingTokens += usage{"output_tokens_details", "thinking_tokens"}.getInt
    result.cacheRead += usage{"cache_read_input_tokens"}.getInt
    result.cacheWrite += usage{"cache_creation_input_tokens"}.getInt
    let content = message{"content"}
    if content.kind == JArray:
      for part in content:
        if part{"type"}.getStr("") != "tool_use":
          continue
        inc result.toolCalls
        if part{"name"}.getStr("") == "Skill":
          let skill = part{"input", "skill"}.getStr("")
          if skill.len > 0 and skill notin result.skills:
            result.skills.add(skill)

proc defaultProjects(): string =
  let base = getEnv("XDG_DATA_HOME", getEnv("HOME", "") & "/.local/share")
  base / "cardinal" / "projects"

proc runSessionMetrics*(args: seq[string]): int =
  let root = if args.len > 0: args[0] else: defaultProjects()
  if not dirExists(root):
    stderr.writeLine("warning: the projects directory " & root & " cannot be read")
    echo Header
    return 0
  var rows: seq[Row]
  for path in walkDirRec(root, yieldFilter = {pcFile}):
    if path.endsWith(".jsonl"):
      rows.add(readSession(path))
  rows.sort(proc (a, b: Row): int =
    if a.hasStart != b.hasStart: (if a.hasStart: -1 else: 1)
    elif a.hasStart: cmp(a.started, b.started)
    else: cmp(a.session, b.session))
  echo Header
  for row in rows:
    var duration = 0'i64
    if row.hasStart:
      duration = (row.ended - row.started).inSeconds
    let cells = [
      row.session,
      (if row.project.len > 0: row.project else: "-"),
      "", "", # issues, revision
      row.models.join(","),
      "", # versions
      row.start,
      $duration,
      $row.turns,
      $row.toolCalls,
      $row.inputTokens,
      $row.outputTokens,
      $row.thinkingTokens,
      $row.cacheRead,
      $row.cacheWrite,
      $row.peak,
      "", "0", "0", "0", # advisor models/calls/input/output
      $row.refusals,
      row.skills.join(","),
      "0", # validations
      $row.other,
      $row.unreadable,
      $row.unknown,
      $row.missing,
    ]
    echo cells.join("\t")
  0
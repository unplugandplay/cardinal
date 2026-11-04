## The settings entries and the native review hook, for `sync` and `check`.
##
## The source fragment `entries/.claude/settings.json` lists the allow/ask/deny
## rules and hook objects Cardinal manages; `sync` merges them into
## `.claude/settings.json`, records them in the lock, and installs the git
## `pre-commit` review hook.

import std/[json, os, strutils]

import config, shell

const
  SettingsPath* = ".claude/settings.json"
  FragmentPath* = "entries/.claude/settings.json"
  HookMarker = "# harness: native review gate"

type
  EntryKind* = enum
    ekAllow, ekAsk, ekDeny, ekHook

  Entry* = object
    kind*: EntryKind
    rule*: string
    event*: string
    matcher*: string
    hook*: JsonNode

const HookScript = """#!/bin/sh
# harness: native review gate (installed by `cardinal sync`, do not edit)
legacy=.claude/hooks/require_code_review.rs
if [ -f "$legacy" ] && command -v rust-script >/dev/null 2>&1; then
    rust-script "$legacy" native || exit $?
elif [ ! -f "$legacy" ] && [ -f .claude/cardinal.toml ]; then
    bin=
    if [ -x ./nim/bin/cardinal-nim ]; then
        bin=./nim/bin/cardinal-nim
    elif command -v cardinal-nim >/dev/null 2>&1; then
        bin=cardinal-nim
    elif command -v cardinal >/dev/null 2>&1; then
        bin=cardinal
    fi
    if [ -z "$bin" ]; then
        echo "cardinal: review gate skipped, cardinal-nim and cardinal are missing" >&2
    else
        "$bin" run review-gate native
        code=$?
        if [ "$code" -eq 3 ]; then
            echo "cardinal: review gate skipped, it could not run it" >&2
        elif [ "$code" -ne 0 ]; then
            exit "$code"
        fi
    fi
else
    echo "cardinal: review gate skipped, its hook or the program to run it is missing" >&2
fi
case "$0" in */*) dir="${0%/*}" ;; *) dir=. ;; esac
user="$dir/pre-commit.user"
if [ -x "$user" ]; then
    exec "$user" "$@"
fi
exit 0
"""

proc listName(kind: EntryKind): string =
  case kind
  of ekAllow: "allow"
  of ekAsk: "ask"
  of ekDeny: "deny"
  of ekHook: "hooks"

proc ofSource*(source: string): seq[Entry] =
  let path = source / FragmentPath
  if not fileExists(path):
    return
  let node = parseJson(readFile(path))
  if node.hasKey("permissions"):
    for kind in [ekAllow, ekAsk, ekDeny]:
      let key = listName(kind)
      if node["permissions"].hasKey(key):
        for rule in node["permissions"][key]:
          result.add Entry(kind: kind, rule: rule.getStr(""))
  if node.hasKey("hooks"):
    for event, groups in node["hooks"].pairs:
      for group in groups:
        var matcher = ""
        if group.hasKey("matcher") and group["matcher"].kind == JString:
          matcher = group["matcher"].getStr("")
        for hook in group["hooks"]:
          result.add Entry(kind: ekHook, event: event, matcher: matcher, hook: hook)

proc toLock*(entry: Entry): string =
  var node = newJObject()
  if entry.kind == ekHook:
    node["event"] = %entry.event
    node["matcher"] = (if entry.matcher.len == 0: newJNull() else: %entry.matcher)
    node["hook"] = entry.hook
  else:
    node[listName(entry.kind)] = %entry.rule
  $node

proc fromLock*(line: string): Entry =
  let node = parseJson(line)
  for kind in [ekAllow, ekAsk, ekDeny]:
    let key = listName(kind)
    if node.hasKey(key):
      return Entry(kind: kind, rule: node[key].getStr(""))
  var matcher = ""
  if node.hasKey("matcher") and node["matcher"].kind == JString:
    matcher = node["matcher"].getStr("")
  Entry(kind: ekHook, event: node{"event"}.getStr(""), matcher: matcher,
        hook: node{"hook"})

proc label*(entry: Entry): string =
  if entry.kind == ekHook:
    let matcher = if entry.matcher.len > 0: entry.matcher & " " else: ""
    "hook " & entry.event & " " & matcher & entry.hook{"command"}.getStr("")
  else:
    listName(entry.kind) & " " & entry.rule

proc allMatchers(matcher: string): bool =
  matcher.len == 0 or matcher == "*"

proc sameMatcher(have, wanted: string): bool =
  have == wanted or (allMatchers(have) and allMatchers(wanted))

proc holds*(settings: JsonNode; entry: Entry): bool =
  if entry.kind != ekHook:
    if not settings.hasKey("permissions") or
       not settings["permissions"].hasKey(listName(entry.kind)):
      return false
    for rule in settings["permissions"][listName(entry.kind)]:
      if rule.getStr("") == entry.rule:
        return true
    return false
  if not settings.hasKey("hooks") or not settings["hooks"].hasKey(entry.event):
    return false
  for group in settings["hooks"][entry.event]:
    var matcher = ""
    if group.hasKey("matcher") and group["matcher"].kind == JString:
      matcher = group["matcher"].getStr("")
    if not sameMatcher(matcher, entry.matcher):
      continue
    for hook in group{"hooks"}:
      if hook == entry.hook:
        return true
  false

proc add*(settings: JsonNode; entry: Entry) =
  if entry.kind != ekHook:
    if not settings.hasKey("permissions"):
      settings["permissions"] = newJObject()
    let key = listName(entry.kind)
    if not settings["permissions"].hasKey(key):
      settings["permissions"][key] = newJArray()
    settings["permissions"][key].add(%entry.rule)
    return
  if not settings.hasKey("hooks"):
    settings["hooks"] = newJObject()
  if not settings["hooks"].hasKey(entry.event):
    settings["hooks"][entry.event] = newJArray()
  for group in settings["hooks"][entry.event]:
    var matcher = ""
    if group.hasKey("matcher") and group["matcher"].kind == JString:
      matcher = group["matcher"].getStr("")
    if sameMatcher(matcher, entry.matcher):
      group["hooks"].add(entry.hook)
      return
  var group = newJObject()
  if entry.matcher.len > 0:
    group["matcher"] = %entry.matcher
  let hooks = newJArray()
  hooks.add(entry.hook)
  group["hooks"] = hooks
  settings["hooks"][entry.event].add(group)

proc remove(settings: JsonNode; entry: Entry): bool =
  if entry.kind != ekHook:
    if not settings.hasKey("permissions") or
       not settings["permissions"].hasKey(listName(entry.kind)):
      return false
    let rules = settings["permissions"][listName(entry.kind)]
    var index = 0
    while index < rules.len:
      if rules[index].getStr("") == entry.rule:
        rules.elems.delete(index)
        return true
      inc index
    return false
  if not settings.hasKey("hooks") or not settings["hooks"].hasKey(entry.event):
    return false
  for group in settings["hooks"][entry.event]:
    var matcher = ""
    if group.hasKey("matcher") and group["matcher"].kind == JString:
      matcher = group["matcher"].getStr("")
    if not sameMatcher(matcher, entry.matcher):
      continue
    let hooks = group{"hooks"}
    var index = 0
    while index < hooks.len:
      if hooks[index] == entry.hook:
        hooks.elems.delete(index)
        return true
      inc index
  false

proc containsEntry(entries: seq[Entry]; entry: Entry): bool =
  for candidate in entries:
    if candidate == entry:
      return true

## Merge the wanted entries into the settings; returns the lines to print and
## whether the settings changed. `settings` is modified in place.
proc merge*(settings: JsonNode; wanted, recorded: seq[Entry]; force: bool;
            outcomes: var seq[string]): bool =
  for entry in recorded:
    if containsEntry(wanted, entry):
      continue
    if remove(settings, entry):
      outcomes.add("removed: " & SettingsPath & " " & label(entry))
      result = true
    else:
      outcomes.add("kept: " & SettingsPath & " " & label(entry))
  for entry in wanted:
    if holds(settings, entry):
      continue
    if containsEntry(recorded, entry) and not force:
      outcomes.add("conflict: " & SettingsPath & " " & label(entry) &
        " (edited locally, fix it in the source or use --force)")
      continue
    add(settings, entry)
    outcomes.add("added: " & SettingsPath & " " & label(entry))
    result = true

proc writeSettings*(path: string; settings: JsonNode) =
  writeFile(path, pretty(settings, indent = 2) & "\n")

proc readSettings*(path: string): JsonNode =
  if not fileExists(path):
    return nil
  try:
    parseJson(readFile(path))
  except CatchableError, ValueError:
    nil

## Install or refresh the native review hook; returns the report line, empty when
## up to date or not a git repository.
proc installReviewHook*(root: string; dryRun: bool): string =
  let hooksRaw = gitPath(root, "hooks")
  if hooksRaw.len == 0:
    return ""
  let hooks = hooksRaw
  let redirected = execCapture("git", @["config", "--get", "core.hooksPath"], root, "").output.strip()
  if redirected.len > 0:
    return "skipped (core.hooksPath is set)"
  let path = hooks / "pre-commit"
  let chained = hooks / "pre-commit.user"
  if fileExists(path):
    let text = readFile(path)
    if not text.contains(HookMarker):
      if not fileExists(chained) and not dryRun:
        moveFile(path, chained)
    elif text == HookScript:
      return ""
  if dryRun:
    return "would install"
  createDir(hooks)
  writeFile(path, HookScript)
  setFilePermissions(path, {fpUserRead, fpUserWrite, fpUserExec,
                            fpGroupRead, fpGroupExec, fpOthersRead, fpOthersExec})
  "added: native review gate"

proc hookInstalled*(root: string): bool =
  let path = gitPath(root, "hooks") / "pre-commit"
  fileExists(path) and readFile(path).contains(HookMarker)
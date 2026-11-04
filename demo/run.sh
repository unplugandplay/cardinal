#!/bin/sh
# End-to-end demo: sync a hello-world project, run every gate and workflow helper, keep the logs.
# Usage: sh demo/run.sh   (logs land in demo/logs/)
set -u
ROOT=$(cd "$(dirname "$0")/.." && pwd)
CARDINAL="${CARDINAL:-$ROOT/nim/bin/cardinal-nim}"
LOGS="$ROOT/demo/logs"
TMP=$(mktemp -d "${TMPDIR:-/tmp}/cardinal-demo.XXXXXX")
FIX="$TMP/fixtures"
export PATH="$HOME/.local/bin:/opt/homebrew/bin:$PATH"
PASS=0
FAIL=0

rm -rf "$LOGS"
mkdir -p "$LOGS" "$FIX"
cleanup() { rm -rf "$TMP"; }
trap cleanup EXIT

record() {
  name=$1
  shift
  { echo "\$ $*"; "$@"; echo "exit=$?"; } > "$LOGS/$name.log" 2>&1
}
record_in() {
  name=$1
  file=$2
  shift 2
  { echo "\$ $* < $file"; "$@" < "$file"; echo "exit=$?"; } > "$LOGS/$name.log" 2>&1
}
code_of() { sed -n 's/^exit=//p' "$LOGS/$1.log" | tail -1; }
expect() {
  desc=$1
  want=$2
  got=$3
  if [ "$got" = "$want" ]; then
    PASS=$((PASS + 1))
    echo "PASS $desc (exit $got)"
  else
    FAIL=$((FAIL + 1))
    echo "FAIL $desc (want $want, got $got; see demo/logs)"
  fi
}

echo "== build and unit test"
record unit-sha256 sh -c "cd '$ROOT' && nim c -r --hints:off -o:nim/bin/sha256_test nim/sha256_test.nim"
expect "sha256 known answers" 0 "$(code_of unit-sha256)"

echo "== the hello-world project"
WORK="$TMP/hello-world"
cp -R "$ROOT/demo/hello-world/." "$WORK/"
python3 - "$WORK/.claude/cardinal.toml" "$ROOT/harness" <<'PY'
import sys
path, source = sys.argv[1], sys.argv[2]
text = open(path).read().replace('source = "../../harness"', f'source = "{source}"')
open(path, "w").write(text)
PY
(cd "$WORK" && git init -q && git config user.email demo@example.com && git config user.name demo)
record sync sh -c "cd '$WORK' && '$CARDINAL' sync"
expect "sync copies and seeds" 0 "$(code_of sync)"
record check sh -c "cd '$WORK' && '$CARDINAL' check"
expect "check after sync" 0 "$(code_of check)"
record dry-run sh -c "cd '$WORK' && '$CARDINAL' sync --dry-run"
expect "sync --dry-run is a no-op" 0 "$(code_of dry-run)"
: > "$WORK/.git/claude-review-waived"
record commit-adopt sh -c "cd '$WORK' && git add .claude AGENTS.md CHANGELOG.md ROADMAP.md justfile cardinal-commits.just cardinal-issues.just cardinal-review.just cardinal-specs.just cardinal.just specs .github && git commit -m 'chore: adopt cardinal'"
expect "the review hook lets a waived commit through" 0 "$(code_of commit-adopt)"

echo "== gates"
python3 - "$FIX" "$WORK" <<'PY'
import json, os, sys
fix, work = sys.argv[1], sys.argv[2]
os.makedirs(fix, exist_ok=True)
def dump(name, value):
    open(os.path.join(fix, name), "w").write(json.dumps(value))
dump("review-bad.json", {"action": "shell", "input": {"command": "git commit --no-verify -m x"}, "cwd": work, "session": "demo-gates"})
dump("review-good.json", {"action": "shell", "input": {"command": "git status"}, "cwd": work, "session": "demo-gates"})
dump("read.json", {"action": "read", "input": {"file_path": os.path.join(work, "big.txt")}, "cwd": work, "session": "demo-read"})
dump("language-fr.json", {"action": "idle", "cwd": work, "stop_active": False, "last_message": "Bonjour, je suis pret. Le fichier a ete modifie et les tests passent maintenant. Vous pouvez relancer la commande pour verifier le resultat.", "session": "demo-lang"})
dump("language-en.json", {"action": "idle", "cwd": work, "stop_active": False, "last_message": "Hello, the file was changed and the tests pass now. You can run the command again to check the result.", "session": "demo-lang"})
dump("idle.json", {"action": "idle", "cwd": work, "stop_active": False, "session": "demo-lint"})
dump("format.json", {"action": "format", "input": {"file_path": os.path.join(work, "hello.nim")}, "cwd": work, "session": "demo-fmt"})
dump("skill.json", {"action": "skill", "input": {"skill": "code-review"}, "success": True, "cwd": work, "session": "demo-review"})
PY
python3 -c "open('$WORK/big.txt','w').write('line of text for the read hint test\n'*900)"

record_in review-bad "$FIX/review-bad.json" "$CARDINAL" run review-gate check
expect "review gate refuses --no-verify" 2 "$(code_of review-bad)"
record_in review-good "$FIX/review-good.json" "$CARDINAL" run review-gate check
expect "review gate allows a normal command" 0 "$(code_of review-good)"
record_in read-first "$FIX/read.json" "$CARDINAL" run read-hint
expect "read hint denies the first whole read" 0 "$(code_of read-first)"
grep -q '"permissionDecision":"deny"' "$LOGS/read-first.log" && expect "read hint answer is a deny JSON" 0 0 || expect "read hint answer is a deny JSON" 0 1
record_in read-second "$FIX/read.json" "$CARDINAL" run read-hint
expect "read hint lets the repeat through" 0 "$(code_of read-second)"
record_in language-fr "$FIX/language-fr.json" "$CARDINAL" run language-hint
expect "language hint refuses French" 2 "$(code_of language-fr)"
record_in language-en "$FIX/language-en.json" "$CARDINAL" run language-hint
expect "language hint allows English" 0 "$(code_of language-en)"
echo "# demo" >> "$WORK/hello.nim"
record_in lint-pass "$FIX/idle.json" "$CARDINAL" run lint-gate
expect "lint gate passes" 0 "$(code_of lint-pass)"
python3 - "$WORK/.claude/cardinal.toml" <<'PY'
import sys
p=sys.argv[1]; s=open(p).read().replace('lint_command = "nim check hello.nim"','lint_command = "false"')
open(p,'w').write(s)
PY
echo "echo 'hello again'" >> "$WORK/hello.nim"
rm -f "$WORK/.git/claude-lint-pass"
record_in lint-fail "$FIX/idle.json" "$CARDINAL" run lint-gate
expect "lint gate refuses a failing lint" 2 "$(code_of lint-fail)"
record_in format "$FIX/format.json" "$CARDINAL" run format-code
expect "format hook runs" 0 "$(code_of format)"
[ -f "$WORK/formatted.log" ] && expect "formatter left its trace" 0 0 || expect "formatter left its trace" 0 1

echo "== workspace lock"
python3 - "$FIX" "$WORK" <<'PY'
import json, sys, os
fix, work = sys.argv[1], sys.argv[2]
for name, session in [("lock-a.json","demo-a"), ("lock-b.json","demo-b")]:
    open(os.path.join(fix,name),"w").write(json.dumps({"action":"lock","session":session,"cwd":work}))
PY
sleep 300 & A=$!
sleep 300 & B=$!
echo dirty > "$WORK/wip"
(cd "$WORK" && CLAUDE_PID=$A "$CARDINAL" run workspace-lock start < "$FIX/lock-a.json" > "$LOGS/lock-start.log" 2>&1)
expect "workspace lock start" 0 "$?"
(cd "$WORK" && CLAUDE_PID=$B "$CARDINAL" run workspace-lock check < "$FIX/lock-b.json" > "$LOGS/lock-check.log" 2>&1)
expect "workspace lock refuses the second session" 2 "$?"
(cd "$WORK" && CLAUDE_PID=$A "$CARDINAL" run workspace-lock release < "$FIX/lock-a.json" > "$LOGS/lock-release.log" 2>&1)
kill $A $B 2>/dev/null
rm -f "$WORK/wip"

echo "== native review gate"
echo new > "$WORK/new.txt"
(cd "$WORK" && "$CARDINAL" run review-gate native > "$LOGS/native-none.log" 2>&1)
expect "native gate refuses without a marker" 2 "$?"
(cd "$WORK" && "$CARDINAL" run review-gate record < "$FIX/skill.json" > "$LOGS/native-record.log" 2>&1)
expect "review-gate record writes the marker" 0 "$?"
(cd "$WORK" && "$CARDINAL" run review-gate native > "$LOGS/native-covered.log" 2>&1)
expect "native gate accepts the reviewed tree" 0 "$?"
echo another > "$WORK/another.txt"
(cd "$WORK" && "$CARDINAL" run review-gate native > "$LOGS/native-unseen.log" 2>&1)
expect "native gate refuses an unseen file" 2 "$?"
rm -f "$WORK/another.txt" "$WORK/.git/claude-code-review.json"
: > "$WORK/.git/claude-review-waived"
(cd "$WORK" && "$CARDINAL" run review-gate native > "$LOGS/native-waiver.log" 2>&1)
expect "native gate honours a waiver" 0 "$?"

echo "== workflow helpers"
printf 'feat: add a demo feature\n' > "$TMP/good-msg"
printf 'Bad Subject.\n\nCo-Authored-By: x <a@b>\n' > "$TMP/bad-msg"
record_in commit-good "$TMP/good-msg" "$CARDINAL" run commit-check
expect "commit-check accepts a good message" 0 "$(code_of commit-good)"
record_in commit-bad "$TMP/bad-msg" "$CARDINAL" run commit-check
expect "commit-check refuses a bad message" 1 "$(code_of commit-bad)"
echo "note" >> "$WORK/README.md"
: > "$WORK/.git/claude-review-waived"
(cd "$WORK" && git add README.md && git commit -m "docs: add a demo note" > "$LOGS/commit-note.log" 2>&1)
(cd "$WORK" && "$CARDINAL" run check-commits HEAD~1 HEAD > "$LOGS/check-commits.log" 2>&1)
expect "check-commits accepts the range" 0 "$?"

mkdir -p "$WORK/issues" "$WORK/issues/done"
cat > "$WORK/issues/demo_issue.md" <<'EOF'
---
priority: high
depends_on: []
why: 'the demo needs one issue'
---
# Demo issue

# Goal
Show the queue.

# Approach
1. Read it.

# Resources
* none
EOF
cat > "$WORK/issues/broken_issue.md" <<'EOF'
---
priority: urgent
---
# Broken
EOF
(cd "$WORK" && "$CARDINAL" run check-issue issues/demo_issue.md > "$LOGS/issue-good.log" 2>&1)
expect "check-issue accepts the template" 0 "$?"
(cd "$WORK" && "$CARDINAL" run check-issue issues/broken_issue.md > "$LOGS/issue-bad.log" 2>&1)
expect "check-issue refuses the broken issue" 1 "$?"
(cd "$WORK" && "$CARDINAL" run next-issue > "$LOGS/next-issue.log" 2>&1)
expect "next-issue runs" 0 "$?"
(cd "$WORK" && "$CARDINAL" run close-issue issues/demo_issue.md > "$LOGS/close-issue.log" 2>&1)
expect "close-issue moves the file" 0 "$?"
[ -f "$WORK/issues/done/demo_issue.md" ] && expect "closed issue is in done/" 0 0 || expect "closed issue is in done/" 0 1
(cd "$WORK" && "$CARDINAL" run review-target HEAD~1 > "$LOGS/review-target.log" 2>&1)
expect "review-target runs" 0 "$?"
(cd "$WORK" && "$CARDINAL" run deliverable-size HEAD~1 > "$LOGS/deliverable-size.log" 2>&1)
expect "deliverable-size runs" 0 "$?"
(cd "$WORK" && "$CARDINAL" run worktree open demo-wt > "$LOGS/worktree-open.log" 2>&1)
expect "worktree open" 0 "$?"
(cd "$WORK" && "$CARDINAL" run worktree close demo-wt > "$LOGS/worktree-close.log" 2>&1)
expect "worktree close" 0 "$?"
(cd "$WORK" && "$CARDINAL" run check-specs > "$LOGS/check-specs.log" 2>&1)
expect "check-specs" 0 "$?"
(cd "$WORK" && "$CARDINAL" run fixed-context > "$LOGS/fixed-context.log" 2>&1)
expect "fixed-context" 0 "$?"
(cd "$WORK" && "$CARDINAL" run audit-sources > "$LOGS/audit-sources.log" 2>&1)
expect "audit-sources" 0 "$?"
(cd "$WORK" && "$CARDINAL" run release > "$LOGS/release-proposal.log" 2>&1)
expect "release proposal" 0 "$?"
(cd "$WORK" && "$CARDINAL" run release 0.1.0 > "$LOGS/release-write.log" 2>&1)
expect "release writes the changelog" 0 "$?"
(cd "$WORK" && "$CARDINAL" run release 0.1.1 --tag > "$LOGS/release-tag.log" 2>&1)
expect "release tags" 0 "$?"

echo "== session metrics"
mkdir -p "$FIX/projects/-demo"
cat > "$FIX/projects/-demo/ses_demo.jsonl" <<'EOF'
{"type":"user","sessionId":"ses_demo","cwd":"/demo/hello-world","timestamp":"2026-11-04T10:00:00.000Z","isSidechain":false,"message":{"role":"user","content":[{"type":"text","text":"hello"}]}}
{"type":"assistant","sessionId":"ses_demo","cwd":"/demo/hello-world","timestamp":"2026-11-04T10:00:01.000Z","isSidechain":false,"message":{"id":"msg_1","role":"assistant","model":"demo/model","usage":{"input_tokens":10,"output_tokens":2,"output_tokens_details":{"thinking_tokens":1},"cache_read_input_tokens":3,"cache_creation_input_tokens":0},"content":[{"type":"text","text":"hello world"}]}}
EOF
record metrics "$CARDINAL" run session-metrics "$FIX/projects"
expect "session-metrics" 0 "$(code_of metrics)"
grep -q "ses_demo" "$LOGS/metrics.log" && expect "session-metrics lists the session" 0 0 || expect "session-metrics lists the session" 0 1

# Make the logs portable: no home, repo or temp paths (the resolved temp path too).
TMP_REAL=$(cd "$TMP" && pwd -P)
for log in "$LOGS"/*.log; do
  sed -e "s|$ROOT|<repo>|g" -e "s|$TMP_REAL|<tmp>|g" -e "s|$TMP|<tmp>|g" -e "s|$HOME|~|g" "$log" > "$log.tmp" && mv "$log.tmp" "$log"
done

echo
echo "passed: $PASS, failed: $FAIL"
[ "$FAIL" -eq 0 ]
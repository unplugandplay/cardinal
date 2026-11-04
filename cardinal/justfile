# Development commands. `just` lists them; the agent harness uses the same entry points.
import 'cardinal.just'
set allow-duplicate-recipes

default:
    @just --list

# Format, lint (warning-free), test, then check the specs; prints nothing on success, since the agent reads all of it.
# Strict: this repository ships the managed tests, so their citations must resolve to a spec here (consumers stay
# lenient).
check: fmt clippy test
    @just check-specs --strict

fmt:
    @cargo fmt --all

clippy:
    @cargo clippy -q --workspace --all-targets -- -D warnings

# The output of the tests, only when they fail.
test:
    #!/usr/bin/env sh
    out=$(cargo test -q --workspace 2>&1) || { printf '%s\n' "$out"; exit 1; }

# Install the `cardinal` binary to ~/.cargo/bin (`--force`: over the installed one, even of the same version).
install:
    cargo install --force --path crates/cardinal --locked

# This repository is its own first consumer: check it with the binary just built.
check-cardinal:
    cargo run -q -p cardinal -- check

# The spec checker of the working tree, whatever `cardinal` is installed (CI installs none for this repository).
check-specs *args:
    @cargo run -q -p cardinal -- run check-specs {{args}}

# Write `version` into the crate and its lock (the `release_version_command` of `just release`).
set-version version:
    sed -i.bak '/^\[package\]/,/^\[/s/^version = ".*"/version = "{{version}}"/' crates/cardinal/Cargo.toml && rm crates/cardinal/Cargo.toml.bak
    cargo update --workspace --offline -q

# Run the issue of the fixture committed at HEAD (`fixtures/replay/`) headless `runs` times against the Cardinal
# revision `rev`, each in a fresh copy kept under /tmp, then grade it: one tab-separated line per run, `graded` pass
# when the run ended in `success`, committed, recorded a code review and passes the held-out test. Billed to the
# user's plan; `claude` names another command for a dry run (it receives the arguments of `claude -p` and prints its
# JSON result).
replay rev="HEAD" runs="1" claude="claude":
    #!/usr/bin/env bash
    set -euo pipefail
    repo=$PWD
    commit=$(git rev-parse --verify "{{rev}}^{commit}")
    claude="{{claude}}"
    case $claude in */*) claude=$(realpath "$claude") ;; esac
    issue=issues/count_the_frequency_of_each_word.md
    printf 'run\tdir\tsession\tsubtype\tduration_s\tcost_usd\tturns\tinput_tokens\toutput_tokens\tcache_read_tokens\tcache_creation_tokens\tcommits\treviews\ttests\tgraded\n'
    for run in $(seq 1 "{{runs}}"); do
        dir=$(mktemp -d /tmp/cardinal-replay.XXXXXX)/replay-fixture
        mkdir "$dir"
        git archive HEAD:fixtures/replay | tar -C "$dir" -xf -
        mv "$dir/grade/grade.rs" "$dir.grade.rs" && rmdir "$dir/grade"
        (
            cd "$dir"
            git init -q -b main
            cardinal init "$repo/.git" --rev "$commit" >/dev/null
            # The setup is committed before the review gate exists, then a second sync installs it (a revision
            # older than the native gate installs none).
            cardinal sync >/dev/null
            gate=$(test -e .git/hooks/pre-commit && echo yes || true)
            rm -f .git/hooks/pre-commit
            git add -- . && git commit -q -m 'chore: set up the replay fixture'
            cardinal sync >/dev/null
            [ -z "$gate" ] || test -x .git/hooks/pre-commit
            sed -i "s/^- Base commit: BASE$/- Base commit: $(git rev-parse HEAD)/" "$issue"
            git commit -q -m 'docs: record the base commit' -- "$issue"
        ) >&2
        base=$(git -C "$dir" rev-parse HEAD)
        # The review lines as `delivered-quality` counts them (not a table row), only those the run adds.
        count_reviews() { cat "$@" 2>/dev/null | grep -i 'code review' | grep -vc '^|' || true; }
        handed=$(count_reviews "$dir/$issue")
        start=$(date +%s)
        (cd "$dir" && "$claude" -p "Work the issue $issue." --output-format json --permission-mode auto \
            --max-budget-usd 5) >"$dir.json" || true
        elapsed=$(( $(date +%s) - start ))
        commits=$(git -C "$dir" rev-list --count "$base..HEAD")
        reviews=$(( $(count_reviews "$dir/$issue" "$dir/issues/done/${issue#issues/}") - handed ))
        mkdir -p "$dir/tests" && cp "$dir.grade.rs" "$dir/tests/replay_grade.rs"
        tests=fail
        (cd "$dir" && cargo test -q >/dev/null 2>&1) && tests=pass
        subtype=$(jq -r '.subtype // ""' "$dir.json" 2>/dev/null || true)
        graded=fail
        [ "$subtype" = success ] && [ "$commits" -gt 0 ] && [ "$reviews" -gt 0 ] && [ "$tests" = pass ] && graded=pass
        jq -er --arg run "$run" --arg dir "$dir" --arg elapsed "$elapsed" --arg commits "$commits" \
            --arg reviews "$reviews" --arg tests "$tests" --arg graded "$graded" \
            '[$run, $dir, .session_id, .subtype, $elapsed, .total_cost_usd, .num_turns, .usage.input_tokens,
              .usage.output_tokens, .usage.cache_read_input_tokens, .usage.cache_creation_input_tokens, $commits,
              $reviews, $tests, $graded] | map(. // "" | tostring) | join("\t")' "$dir.json" 2>/dev/null \
            || printf '%s\t%s\t\tunreadable\t%s\t\t\t\t\t\t\t%s\t%s\t%s\t%s\n' "$run" "$dir" "$elapsed" "$commits" "$reviews" "$tests" "$graded"
    done

# Bring this repository in step with its own `files/` and `seed/`.
sync *args:
    cargo run -q -p cardinal -- sync {{args}}

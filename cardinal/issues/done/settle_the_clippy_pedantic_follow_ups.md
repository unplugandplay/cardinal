# Settle the clippy pedantic follow-ups

# Goal
The `high` review of `rename_the_harness_named_tools_to_cardinal` found in commit `e5352ff` (clippy pedantic) a
change of behaviour at the edges and three fixes that moved a problem instead of removing it. Settle each one.

# Findings
* `crates/cardinal/src/tools/cardinal_report.rs:576`: `av == bv` became `av.total_cmp(&bv).is_eq()`. `total_cmp`
  orders `-0.0` before `0.0` and makes `NaN` equal to itself, so a `-0.00` after `0.00` reads `helped` or `hurt`
  instead of `no observed effect`. The metrics are counts and ratios of counts, so `-0.00` should not occur; not
  observed.
* `crates/cardinal/src/tools/check_specs.rs:270`: the split of `parse` declares the same `problem` closure
  (`out.push(format!("{file}: {m}"))`) twice; the two copies can drift.
* `crates/cardinal/src/tools/review_gate.rs:90`: `batch.iter().map(|n| [n, "\n"].concat()).collect::<String>()`
  still allocates one `String` per name; one buffer does it.
* `crates/cardinal/src/budget.rs:36`: an `expect("three parts")` became a `let … else { return Vec::new() }`: a
  broken invariant now drops the warning of SYN-38 silently instead of failing in the tests.

# Approach
1. `total_cmp`: restore `av == bv` under `#[allow(clippy::float_cmp)]` with its reason, or keep it
   and say why.
2. One shared helper for `problem`; `join` or `push_str` in `review_gate`; restore a loud failure in `budget`
   (`unreachable!` or `debug_assert!`), each under `just check`.

# Resources
* `git show e5352ff`
* `crates/cardinal/src/tools/cardinal_report.rs`, `check_specs.rs`, `review_gate.rs`, `crates/cardinal/src/budget.rs`

# Tracking

Plan: validated

## Understanding
Undo the four side effects of `e5352ff` without losing clippy pedantic. No spec change: VRD-6 already says `equal`,
the fix makes the code match it again. In `budget`, the means differs: fold over the fixed array of three parts, so
no `Option` and no invariant to break, instead of restoring a panic.

## Plan
1. [x] Verdict equality: extract the verdict of two values into a function comparing with `==` under
   `#[allow(clippy::float_cmp)]` and its reason; verified by a unit test (`-0.0` after `0.0` is `no observed effect`);
   makes true VRD-6 at the edge (~30 lines).
2. [x] Pedantic cleanups: one `problem` helper shared by `parse` and `parse_requirements`; one buffer in
   `untracked_hashes`; the largest part of `budget` taken without an `Option`; verified by `just check` (existing
   tests of check-specs, review-gate, SYN-38) (~50 lines).

3. [x] Fixes of the `high` review: `metric_value` keeps only a finite value and returns it parsed, so `NaN` reads
   `no data` and the verdict parses once; `problem` takes `impl Display` and formats the two spec problems of
   `main` too; verified by a unit test of `metric_value` and `just check`; makes true VRD-6 (~40 lines).

## Decisions
- Mode: autonomous
- Base commit: 1a34320a248e9a19920f07f6839020f854d0df7a
- `budget`: fold over the three parts, no `Option` (answer 1 a).

## Deliverable 1
- done: `moved` gives the verdict of two values, equal under `==` (`float_cmp` allowed with its reason).
- | check | result |
  |---|---|
  | tests, lint, specs | `just check` green |
  | docs | none changed: VRD-6 already says equal |
  | code review | low, see below |
- red: VRD-6: `assertion left == right failed, left: "hurt", right: "no observed effect"`
- Code review: no findings
- not verified: none
- decisions needed: none

## Deliverable 2
- done: one `problem` function in `check_specs`; one buffer for the listing of `review_gate`; the largest part of
  `budget` folded over the three parts, no `Option` left.
- | check | result |
  |---|---|
  | tests, lint, specs | `just check` green |
  | docs | none changed: no behaviour change |
  | code review | low, see below |
- red: none
- Code review: no findings
- not verified: none
- decisions needed: none

## Deliverable 3 (fixes of the `high` review)
- `high` review of `1a34320..62435df`: 4 findings, all fixed here: `NaN` before and after read `helped` or `hurt`
  under `==` (a value that is not finite is now missing: `no data`); two spec problems of `main` formatted by hand;
  `problem(&format!(..))` allocated twice; the verdict parsed each value twice behind a dead `unwrap_or(0.0)`.
- done: `metric_value` returns the printed value and its finite number; `problem` takes `impl Display`, used by
  `main` too.
- | check | result |
  |---|---|
  | tests, lint, specs | `just check` green |
  | docs | none changed: VRD-6 already says a missing value is `no data` |
  | code review | low, see below |
- red: VRD-6: `expected Option<String>, found Option<(String, {float})>` (the test asked for the parsed number)
- Code review: no findings
- not verified: none
- decisions needed: none

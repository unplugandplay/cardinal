---
id: TRC
status: accepted
adrs: []
---

# Boundary proofs of the specs

## Intent

A spec and the tests that trace it are meant to survive a rewrite of the code, so a requirement is proven by a test
that drives the program from its boundary (command line, files, exit code, output). `check-specs` tells such a test
from an internal one by where it sits: a test under a `src` directory calls the code from inside; a test elsewhere
(`tests/`, the managed tests under `.claude/`) drives the boundary. A boundary proof is preferred when it is cheap and
stable, not required: an internal test is the proof when its citation gives the reason, `// spec: ABC-1 (internal:
<reason>)`. The checker reports the requirements whose internal proofs give no reason, as a warning.

## Requirements

- **TRC-1** (deterministic) For each spec holding requirements cited only by tests under a `src` directory, one of those citations at least giving no reason (TRC-6), the checker must print one warning to stderr naming the spec file, the count and the ids of those requirements, without failing.
- **TRC-2** (deterministic) The checker must not list in that warning a requirement that a test outside every `src` directory also cites.
- **TRC-3** (deterministic) The checker must not list in that warning a requirement that no test cites.
- **TRC-4** (deterministic) The checker must not list in that warning a withdrawn requirement.
- **TRC-5** (deterministic) The checker must count a test under `.claude/` as outside every `src` directory, whatever its path below.
- **TRC-6** (deterministic) The checker must not list in that warning a requirement whose every citation under a `src` directory carries a reason, the text `(internal: <reason>)` after its ids with a reason that is not blank.
- **TRC-7** (deterministic) The checker must report as a problem, naming its file and line, a citation whose `(internal: ...)` reason is blank, does not end the line with its `)`, or follows no id.
- **TRC-8** (deterministic) The checker must read from a citation with a reason the same requirement ids as from that citation without it.
- **TRC-9** (deterministic) The checker must read the citations of every `.rs` file under the root, whatever its directory (a crate at the root, `src/` and `tests/`, as well as `crates/` and `.claude/`), except under a directory named `.git` or `node_modules`, or a `target` directory beside a `Cargo.toml` (the build output).
- **TRC-10** (deterministic) The checker must not read the `.rs` files under a directory named like one of the `skipped_dirs` of `.claude/cardinal.toml`.
- **TRC-11** (deterministic) When `.claude/cardinal.toml` sets `spec_files`, the checker must read, instead of the `.rs` files, the files whose name matches one of its patterns, with the exclusions of TRC-9 and TRC-10; a pattern matches the file name alone, `*` standing for any run of characters and every other character for itself.
- **TRC-12** (deterministic) The checker must read as a citation a line whose trimmed text starts with the line-comment marker of its file's extension followed by ` spec:`: `//` for `rs`, `ts`, `tsx`, `js`, `jsx`, `mjs`, `cjs`, `go`, `java`, `kt`, `kts`, `scala`, `swift`, `c`, `cc`, `cpp`, `h`, `hpp`, `cs`, `dart` and `php`; `#` for `py`, `sh`, `bash`, `zsh`, `bats`, `rb`, `pl`, `r` (either case), `ex`, `exs` and `nix`; `--` for `lua`, `sql`, `hs` and `elm`; `;`, once or more, for `clj`, `cljs`, `lisp`, `el`, `scm` and `rkt`; any of the four for another extension.
- **TRC-13** (deterministic) The checker must report as a problem, naming its file and line, a citation of a file other than a `.rs` one whose next line, past the comment lines (any marker of TRC-12) and the decorator lines (trimmed text starting with `@`), is blank or missing; a citation of a `.rs` file keeps the rule of `specs/README.md`, a `fn` past the attributes and comments.
- **TRC-14** (deterministic) When `.claude/cardinal.toml` sets `internal_dirs`, the checker must count as calling the code from inside (TRC-1 to TRC-6) the tests under a directory named like one of its entries, instead of those under a `src` directory.
- **TRC-15** (deterministic) The checker must report as a problem a pattern of `spec_files` or an entry of `internal_dirs` that holds a `/` or a `\`, naming it.
- **TRC-16** (deterministic) The checker must report as a problem a `spec_files` or an `internal_dirs` of `.claude/cardinal.toml` that is not a list of strings, naming the key.

## Scenarios

- (TRC-1) Given `SYN-2` cited only in `crates/cardinal/src/tests.rs`, when the checker runs, then stderr holds one line
  naming `specs/sync.md`, `1` and `SYN-2`, and the exit code is the one the other checks give.
- (TRC-2) Given `SYN-1` cited in `crates/cardinal/src/tests.rs` and in `crates/cardinal/tests/sync_cli.rs`, when the
  checker runs, then `SYN-1` is not in the warning.
- (TRC-1) Given a spec whose cited requirements all have a test outside `src`, when the checker runs, then it prints
  no boundary warning for that spec.
- (TRC-6) Given `SYN-2` cited in `crates/cardinal/src/a.rs` with `// spec: SYN-2 (internal: needs a fake clock)` and in
  `crates/cardinal/src/b.rs` with `// spec: SYN-2`, when the checker runs, then `SYN-2` is in the warning.
- (TRC-7) Given `// spec: SYN-2 (internal: )`, when the checker runs, then it fails naming the file and line.
- (TRC-7) Given `// spec: SYN-2 (internal: slow) SYN-3`, `// spec: SYN-2 (internal: slow` or
  `// spec: (internal: slow)`, when the checker runs, then it fails naming the file and line.
- (TRC-9) Given a crate at the root, `SYN-1` cited in `src/lib.rs` and `SYN-2` in `tests/cli.rs`, when the checker
  runs, then both are traced and the boundary warning names `SYN-1` alone.
- (TRC-9) Given `// spec: SYN-1` in `build.rs` or `examples/e.rs` above no `fn`, when the checker runs, then it fails
  naming the file and line, as anywhere else.
- (TRC-9) Given a `Cargo.toml` at the root and `// spec: SYN-1` in `target/debug/build/x/out/t.rs` only, when the
  checker runs outside git, then `SYN-1` is not traced; in `crates/a/tests/target/t.rs`, a module, it is.
- (TRC-10) Given `skipped_dirs = ["vendor"]` and `// spec: SYN-9` in `vendor/a/tests/t.rs`, when the checker runs, then
  it reports no unknown id.
- (TRC-8) Given `// spec: SYN-2, SYN-3 (internal: the SYN-9 fixture is slow)`, when the checker runs, then it reads
  `SYN-2` and `SYN-3`, flags no unknown id, and `SYN-9` is not cited.

- (TRC-11) Given `spec_files = ["test_*.py"]`, `// spec: SYN-1` in `src/lib.rs` and `# spec: SYN-2` above `def
  test_x():` in `tests/test_cli.py`, when the checker runs, then `SYN-2` is traced and `SYN-1` is not.
- (TRC-11) Given `spec_files = ["*_test.go"]` and `// spec: SYN-1` above `func TestX(t *testing.T) {` in
  `pkg/a/x_test.go`, when the checker runs, then `SYN-1` is traced.
- (TRC-11) Given no `spec_files` key, when the checker runs, then it reads the `.rs` files as TRC-9 says.
- (TRC-12) Given `spec_files = ["*.rs", "*.py"]` and a line `# spec: SYN-1` in a `.rs` file, when the checker runs,
  then `SYN-1` is not cited by that line.
- (TRC-12) Given `-- spec: SYN-1` in `t/a.foo` matched by `spec_files = ["*.foo"]`, when the checker runs, then
  `SYN-1` is cited.
- (TRC-13) Given `# spec: SYN-1`, then `@pytest.mark.slow`, then `def test_x():` in a `.py` test file, when the
  checker runs, then it reports no problem; given `# spec: SYN-1` on the last line, or followed by a blank line, it
  fails naming the file and line.
- (TRC-14) Given `internal_dirs = ["lib"]`, `SYN-1` cited only in `lib/a_test.py` and `SYN-2` only in `src/b_test.py`,
  when the checker runs, then the boundary warning names `SYN-1` alone.
- (TRC-15) Given `spec_files = ["tests/*.py"]`, when the checker runs, then it fails naming `tests/*.py`; given
  `internal_dirs = ["tests/unit"]`, it fails naming `tests/unit`.
- (TRC-16) Given `spec_files = "test_*.py"`, when the checker runs, then it fails naming `spec_files`.
- (TRC-11) Given `spec_files = ["t[1]_?.py"]`, when the checker runs, then it reads `t[1]_?.py` and not `t1_a.py`.
- (TRC-12) Given `;; spec: SYN-1` above `(deftest a ...)` in a `.clj` test file, when the checker runs, then `SYN-1` is
  cited.

## Non-goals

- Checking that a test outside `src` really runs the program: the place of the test is the convention.
- Failing the check on an internal-only proof: a boundary proof is an improvement, not a rule.
- Judging whether a reason is good: the checker only requires one.
- The other format and traceability checks of `check-specs`.
- Telling the build output in another place than a `target` beside a `Cargo.toml` (a custom `target-dir`), or `.rs`
  fixtures, from the tests: the `skipped_dirs` of `.claude/cardinal.toml` leave them out.
- The build output of the other stacks (`dist`, `build`, `.venv`, `__pycache__`): git ignores it, and outside git the
  `skipped_dirs` leave it out.
- Telling a test declaration from other code in a file other than a `.rs` one: the line after the citation only has
  to be there.
- Block comments (`/* */`, `"""`) as citations: the citation is one line comment.

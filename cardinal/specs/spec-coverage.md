---
id: COV
status: accepted
adrs: [0009]
---

# Report of the tests no spec covers

## Intent

A capability gets a spec only when an issue changes it, so the older ones have tests that no spec states. The report
lists the test files that cite no requirement of an `accepted` spec, largest first, so the user ranks the capabilities
worth a spec read back from their code and tests (`skill:write-spec`).

## Requirements

- **COV-1** (deterministic) `cardinal run spec-coverage` must print one line for each test file, a `.rs` file `check-specs` reads (TRC-9, TRC-10) holding a test attribute (`#[test]`, or one whose path ends in `::test` such as `#[tokio::test]`), that cites no requirement of an `accepted` spec.
- **COV-2** (deterministic) The report must not list a test file holding a `// spec:` citation of a requirement of an `accepted` spec.
- **COV-3** (deterministic) The report must read a citation of a withdrawn requirement, of a requirement of a `draft` spec, or of an id no spec defines, as no citation.
- **COV-4** (deterministic) Each line must give the line count of the file, a tab, then its path relative to the root.
- **COV-5** (deterministic) The report must order its lines by line count, largest first, then by path.
- **COV-6** (deterministic) In a git repository, the report must read only the files git lists, so never those of a nested worktree.
- **COV-7** (deterministic) The report must print `no test file without an accepted spec` when it finds test files and lists none of them.
- **COV-8** (deterministic) The report must exit 0 whatever it lists.
- **COV-9** (deterministic) The report must print `no test file found` when it finds no test file.
- **COV-10** (deterministic) The report must exit 1, with git's message on stderr, when git cannot list the files of the repository.
- **COV-11** (deterministic) The report must list the test files of a crate at the root, under `src/` and `tests/`.
- **COV-12** (deterministic) The report must count as a test file every file other than a `.rs` one that `check-specs` reads (TRC-11), whatever it holds.

## Scenarios

- (COV-1) Given `crates/a/src/lib.rs` holding `fn helper() {}` and no `#[test]`, when the report runs, then it does not
  list it.
- (COV-2) Given `crates/a/tests/cli.rs` with two tests, one cited `// spec: RER-1` of an accepted spec and one not
  cited, when the report runs, then it does not list the file: the report ranks files, not tests.
- (COV-3) Given `crates/a/tests/cli.rs` citing only `// spec: DRF-1` of a `draft` spec, when the report runs, then it
  lists the file.
- (COV-9) Given a root with no `.rs` file holding a test attribute (a wrong root), when the report runs, then it prints
  `no test file found`, not that every test file is covered.
- (COV-11) Given `src/lib.rs` and `tests/cli.rs` at the root, each holding a `#[test]` and no citation, when the report
  runs, then it lists both.
- (COV-6) Given a nested worktree `.claude/worktrees/x/crates/a/tests/t.rs` holding a `#[test]`, when the report runs
  in the git repository, then it does not list it.
- (COV-12) Given `spec_files = ["*.rs", "test_*.py"]`, `tests/test_cli.py` holding no citation and `src/lib.rs`
  holding no test attribute, when the report runs, then it lists `tests/test_cli.py` alone.

## Non-goals

- Mapping a source module to the tests that drive it: the test file is the unit.
- Telling the tests of a file that a spec covers from those it does not.
- Judging whether a citation states what its test proves.
- Failing a check: the report ranks, it never blocks.
- Scanning the files `check-specs` does not read: both read the same files.
- Validating the specs it reads: `check-specs` does.

//! The test files that cite no requirement of an `accepted` spec, largest first, to rank the capabilities worth a
//! spec read back from their code and tests (`skill:write-spec`).
//! Usage: `just spec-coverage` (from the repo root; an optional argument is another root). Prints `<lines>\t<path>`
//! per file and exits 0: a report, not a check (COV-8); it exits 1 only when git cannot list the files (COV-10). The
//! files and citations are those `check-specs` reads.

use super::check_specs::{citations, read_specs, test_files};

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

/// What the report found under the root.
enum Found {
    /// No test file at all (COV-9).
    NoTest,
    /// The `(line count, path)` of each uncovered test file, largest first (COV-1 to COV-5).
    Uncovered(Vec<(usize, String)>),
}

fn uncovered(root: &Path) -> Result<Found, String> {
    let mut errors = Vec::new();
    let files = test_files(root, &mut errors);
    if !errors.is_empty() {
        return Err(errors.join("\n"));
    }
    // the problems and warnings of the specs and of the citations are `check-specs`' business
    let live: BTreeSet<String> = read_specs(root, &mut Vec::new(), &mut Vec::new())
        .into_iter()
        .filter(|s| s.status == "accepted")
        .flat_map(|s| {
            s.requirements
                .into_iter()
                .filter(|r| !r.exempt)
                .map(|r| r.id)
        })
        .collect();
    let covered: BTreeSet<String> = citations(root, &files, &mut Vec::new())
        .into_iter()
        .filter(|(id, _)| live.contains(id))
        .flat_map(|(_, cites)| cites.into_iter().map(|c| c.file))
        .collect();
    let tests: Vec<(usize, String)> = files.iter().filter_map(|f| test_file(root, f)).collect();
    if tests.is_empty() {
        return Ok(Found::NoTest);
    }
    let mut found: Vec<(usize, String)> = tests
        .into_iter()
        .filter(|(_, name)| !covered.contains(name))
        .collect();
    found.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.cmp(&b.1)));
    Ok(Found::Uncovered(found))
}

/// The line count and relative path of `file` when it is a test file: a `.rs` one holding a test attribute,
/// `#[test]` or one whose path ends in `::test` (`#[tokio::test]`), arguments allowed; any other file `check-specs`
/// reads (COV-12).
fn test_file(root: &Path, file: &Path) -> Option<(usize, String)> {
    let text = std::fs::read_to_string(file).ok()?;
    let rust = file.extension().is_some_and(|e| e == "rs");
    let is_test = |line: &str| {
        line.trim()
            .strip_prefix("#[")
            .and_then(|a| a.split(['(', ']']).next())
            .is_some_and(|path| path.trim() == "test" || path.trim().ends_with("::test"))
    };
    if rust && !text.lines().any(is_test) {
        return None;
    }
    let name = file
        .strip_prefix(root)
        .unwrap_or(file)
        .display()
        .to_string();
    Some((text.lines().count(), name))
}

pub(super) fn main(args: &[String]) -> u8 {
    if args.len() > 1 || args.iter().any(|a| a.starts_with('-')) {
        eprintln!("usage: cardinal run spec-coverage [root]");
        return 2;
    }
    let root = PathBuf::from(args.first().map_or(".", String::as_str));
    match uncovered(&root) {
        Ok(Found::NoTest) => println!("no test file found"),
        Ok(Found::Uncovered(found)) if found.is_empty() => {
            println!("no test file without an accepted spec");
        }
        Ok(Found::Uncovered(found)) => found
            .iter()
            .for_each(|(lines, file)| println!("{lines}\t{file}")),
        Err(e) => {
            eprintln!("{e}");
            return 1;
        }
    }
    0
}

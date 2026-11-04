//! Print the evolution of Cardinal by revision (`specs/cardinal-report.md`): one markdown table per axis, one row
//! per group of consecutive revisions holding at least `--min-sessions` sessions, with the model, version and project
//! mix and the sample size next to the numbers. Reads the outputs of the other transcript readers, never the
//! transcripts; it reports trends, never a pass or a fail.
//! Usage: `cardinal run cardinal-report --sessions <session-metrics rows> [--breaches <rule-breaches --summary>]
//! [--quality <delivered-quality --summary>]... [--context <fixed-context rows>] [--changes <cardinal-changes rows>]
//! [--min-sessions <n>] [--min-issues <n>] [--by revision|setup] [--last <n>] [--anonymous]`. With `--last <n>`, the
//! tables print only the last `n` groups, the verdicts still reading them all. With `--by setup`, a group is a
//! setup (the main model, alone or advised by another) instead of revisions. With `--changes`, a last table gives
//! each recorded change of Cardinal a verdict from the groups before and after it (`specs/cardinal-verdicts.md`).

use std::collections::{BTreeMap, HashMap, HashSet};

use super::cardinal_changes::{HEADER as CHANGES, RULE_METRIC, WASTE_METRIC};
use super::delivered_quality::{setup_name, SUMMARY_HEADER as QUALITY, SYNTHETIC};
use super::session_metrics::HEADER as SESSIONS;

const BREACHES: &str = "rule\trevision\tapplied\tbreached\tbreaches\tblocked\tbytes";
const CONTEXT: &str =
    "project\trevision\tcommit\tclaude_md_bytes\tagents_md_bytes\tskill_description_bytes\tskills";
const USAGE: &str = "usage: cardinal run cardinal-report --sessions <file> [--breaches <file>] [--quality <file>]... [--context <file>] [--changes <file>] [--min-sessions <n>] [--min-issues <n>] [--by revision|setup] [--last <n>] [--anonymous]";
const TOO_FEW: &str = "too few";

type Row = HashMap<String, String>;

/// The rows of a TSV file whose header must be `header`, each a map from column to value.
fn read(path: &str, header: &str) -> Result<Vec<Row>, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("cannot read {path}: {e}"))?;
    let mut lines = text.lines();
    if lines.next() != Some(header) {
        return Err(format!(
            "{path} has other columns than expected: give it the output of the matching script"
        ));
    }
    let columns: Vec<&str> = header.split('\t').collect();
    Ok(lines
        .filter(|l| !l.is_empty())
        .map(|l| {
            columns
                .iter()
                .map(std::string::ToString::to_string)
                .zip(l.split('\t').map(str::to_string))
                .collect()
        })
        .collect())
}

fn value<'a>(row: &'a Row, column: &str) -> &'a str {
    row.get(column).map(String::as_str).unwrap_or_default()
}

fn number(row: &Row, column: &str) -> u64 {
    value(row, column).parse().unwrap_or(0)
}

/// A revision as the report names it: a 40-digit hash, after `self:` or not, cut to its first 7 digits.
fn short(revision: &str) -> String {
    let (prefix, hash) = revision
        .strip_prefix("self:")
        .map_or(("", revision), |h| ("self:", h));
    if hash.len() == 40 && hash.chars().all(|c| c.is_ascii_hexdigit()) {
        format!("{prefix}{}", &hash[..7])
    } else {
        revision.to_string()
    }
}

/// Consecutive revisions read together, so a comparison rests on enough sessions; or, by setup, the sessions of one
/// setup over any revisions.
struct Group<'a> {
    name: String,
    /// The revisions of the group; none by setup, whose rows are matched by setup instead.
    revisions: HashSet<&'a str>,
    sessions: Vec<&'a Row>,
    too_few: bool,
    by_setup: bool,
    /// Before the last groups `--last` prints: computed for the verdicts, not printed (REP-17).
    hidden: bool,
}

/// The group of the revisions of `open`, in order, with their sessions.
fn close<'a>(open: Vec<(&'a str, Vec<&'a Row>)>, too_few: bool) -> Group<'a> {
    let (first, last) = (short(open[0].0), short(open[open.len() - 1].0));
    let name = if open.len() == 1 {
        first
    } else {
        format!("{first}..{last}")
    };
    Group {
        name,
        revisions: open.iter().map(|(r, _)| *r).collect(),
        sessions: open.into_iter().flat_map(|(_, s)| s).collect(),
        too_few,
        by_setup: false,
        hidden: false,
    }
}

/// The main model of a session, alone or advised by its first advisor model; none without a model (REP-15).
fn setup(row: &Row) -> Option<String> {
    let main = value(row, "models")
        .split(',')
        .find(|m| !m.is_empty() && *m != SYNTHETIC)?;
    let advisor = value(row, "advisor_models").split(',').next();
    Some(setup_name(main, advisor.filter(|a| !a.is_empty())))
}

/// The setups of the sessions of a non-empty revision, most sessions first, then by name (REP-14).
fn setup_groups(sessions: &[Row], min: usize) -> Vec<Group<'_>> {
    let mut by_setup: BTreeMap<String, Vec<&Row>> = BTreeMap::new();
    for row in sessions.iter().filter(|r| !value(r, "revision").is_empty()) {
        if let Some(setup) = setup(row) {
            by_setup.entry(setup).or_default().push(row);
        }
    }
    let mut ordered: Vec<_> = by_setup.into_iter().collect();
    ordered.sort_by(|a, b| b.1.len().cmp(&a.1.len()).then(a.0.cmp(&b.0)));
    ordered
        .into_iter()
        .map(|(name, sessions)| Group {
            name,
            revisions: HashSet::new(),
            too_few: sessions.len() < min,
            sessions,
            by_setup: true,
            hidden: false,
        })
        .collect()
}

/// The revisions in the order of their first session, cut into groups closed at `min` sessions (REP-4).
fn groups(sessions: &[Row], min: usize) -> Vec<Group<'_>> {
    let mut by_revision: HashMap<&str, (&str, Vec<&Row>)> = HashMap::new();
    for row in sessions.iter().filter(|r| !value(r, "revision").is_empty()) {
        let entry = by_revision
            .entry(value(row, "revision"))
            .or_insert((value(row, "start"), Vec::new()));
        entry.0 = entry.0.min(value(row, "start"));
        entry.1.push(row);
    }
    let mut ordered: Vec<_> = by_revision.into_iter().collect();
    ordered.sort_by(|a, b| (a.1 .0, a.0).cmp(&(b.1 .0, b.0)));
    let mut groups: Vec<Group> = Vec::new();
    let mut open: Vec<(&str, Vec<&Row>)> = Vec::new();
    for (revision, (_, rows)) in ordered {
        open.push((revision, rows));
        if open.iter().map(|(_, s)| s.len()).sum::<usize>() >= min {
            groups.push(close(std::mem::take(&mut open), false));
        }
    }
    if !open.is_empty() {
        groups.push(close(open, true));
    }
    groups
}

/// The middle value, the mean of the two middle ones rounded half up for an even count; 0 for none.
fn median(mut values: Vec<u64>) -> u64 {
    values.sort_unstable();
    let n = values.len();
    match n {
        0 => 0,
        _ if n % 2 == 1 => values[n / 2],
        _ => (values[n / 2 - 1] + values[n / 2]).div_ceil(2),
    }
}

/// `name=n` for each name of a comma-separated `column`, `n` the sessions that hold it, most frequent first.
fn mix(sessions: &[&Row], column: &str) -> String {
    sorted_pairs(name_counts(sessions, column))
}

/// The sessions that hold each name of a comma-separated `column`.
fn name_counts<'a>(sessions: &[&'a Row], column: &str) -> BTreeMap<&'a str, u64> {
    let mut counts: BTreeMap<&str, u64> = BTreeMap::new();
    for row in sessions {
        let names: HashSet<&str> = value(row, column)
            .split(',')
            .filter(|n| !n.is_empty())
            .collect();
        for name in names {
            *counts.entry(name).or_default() += 1;
        }
    }
    counts
}

/// `name=n, …`, most frequent first, then by name.
fn sorted_pairs(counts: BTreeMap<&str, u64>) -> String {
    let mut pairs: Vec<_> = counts.into_iter().collect();
    pairs.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(b.0)));
    pairs
        .iter()
        .map(|(name, n)| format!("{name}={n}"))
        .collect::<Vec<_>>()
        .join(", ")
}

/// The cells of a printed table: its columns, and per group its rows, computed for a `too few` group too.
struct Table {
    title: &'static str,
    columns: Vec<String>,
    rows: Vec<Vec<Vec<String>>>,
}

impl Table {
    /// The cell of `column` in the first row of group `g` whose `key` column is `key` (any row with no key).
    fn cell(&self, g: usize, column: &str, key: Option<(&str, &str)>) -> Option<&str> {
        let at = |name: &str| self.columns.iter().position(|c| c == name);
        let index = at(column)?;
        let key = match key {
            Some((name, value)) => Some((at(name)?, value)),
            None => None,
        };
        self.rows
            .get(g)?
            .iter()
            .find(|row| key.is_none_or(|(i, v)| row.get(i).is_some_and(|c| c == v)))?
            .get(index)
            .map(String::as_str)
    }
}

/// A markdown table under `## <title>`: `group`, `sessions`, then `columns`, one row or more per group; a group with
/// too few sessions is one row showing only its size. Returns the cells, for the verdicts.
fn table(
    title: &'static str,
    columns: &[&str],
    groups: &[Group],
    cells: impl Fn(&Group) -> Vec<Vec<String>>,
) -> Table {
    println!("## {title}\n");
    println!("| group | sessions | {} |", columns.join(" | "));
    println!("|{}", "---|".repeat(columns.len() + 2));
    let computed: Vec<Vec<Vec<String>>> = groups.iter().map(&cells).collect();
    let too_few = vec![vec![TOO_FEW.to_string(); columns.len()]];
    for (group, cells) in groups.iter().zip(&computed).filter(|(g, _)| !g.hidden) {
        let rows = if group.too_few { &too_few } else { cells };
        for row in rows {
            println!(
                "| {} | {} | {} |",
                group.name,
                group.sessions.len(),
                row.join(" | ")
            );
        }
    }
    println!();
    Table {
        title,
        columns: columns.iter().map(|c| (*c).to_string()).collect(),
        rows: computed,
    }
}

fn efficiency(groups: &[Group], anonymous: bool) -> Table {
    let mut columns = vec![
        "issues",
        "projects",
        "models",
        "versions",
        "turns",
        "tool_calls",
        "output_tokens",
        "input_tokens",
        "cache_read_share",
        "main_context_peak",
        "peak_sessions",
        "advisor_models",
        "advisor_sessions",
        "advisor_calls",
        "advisor_input_tokens",
        "advisor_output_tokens",
        "validations",
        "other_messages",
    ];
    if groups.iter().any(|g| g.by_setup) {
        columns.insert(4, "revisions");
    }
    table("Efficiency", &columns, groups, |g| {
        let issues: HashSet<(&str, &str)> = g
            .sessions
            .iter()
            .flat_map(|r| {
                value(r, "issues")
                    .split(',')
                    .filter(|i| !i.is_empty())
                    .map(|i| (value(r, "project"), i))
            })
            .collect();
        let projects = if anonymous {
            g.sessions
                .iter()
                .map(|r| value(r, "project"))
                .collect::<HashSet<_>>()
                .len()
                .to_string()
        } else {
            mix(&g.sessions, "project")
        };
        let per_session =
            |f: &dyn Fn(&Row) -> u64| median(g.sessions.iter().map(|r| f(r)).collect()).to_string();
        let column = |c: &'static str| per_session(&move |r: &Row| number(r, c));
        let peaks = main_context_peaks(&g.sessions);
        let peak_sessions = peaks.len().to_string();
        let calls = filled(&g.sessions, "advisor_calls");
        let advised = |c: &str| median_or_empty(filled(&g.sessions, c));
        let mut row = vec![
            issues.len().to_string(),
            projects,
            mix(&g.sessions, "models"),
            mix(&g.sessions, "versions"),
            column("turns"),
            column("tool_calls"),
            column("output_tokens"),
            per_session(&input_sum),
            cache_read_share(&g.sessions),
            median_or_empty(peaks),
            peak_sessions,
            mix(&g.sessions, "advisor_models"),
            calls.len().to_string(),
            median_or_empty(calls),
            advised("advisor_input_tokens"),
            advised("advisor_output_tokens"),
            column("validations"),
            column("other_messages"),
        ];
        if g.by_setup {
            row.insert(4, revision_mix(&g.sessions));
        }
        vec![row]
    })
}

/// `revision=n` for the revisions of `sessions`, named as REP-4 names them (REP-16).
fn revision_mix(sessions: &[&Row]) -> String {
    let mut counts: BTreeMap<String, u64> = BTreeMap::new();
    for row in sessions {
        *counts.entry(short(value(row, "revision"))).or_default() += 1;
    }
    sorted_pairs(counts.iter().map(|(name, n)| (name.as_str(), *n)).collect())
}

/// The input of a session: its uncached input, cache read and cache creation tokens (REP-5).
fn input_sum(row: &Row) -> u64 {
    number(row, "input_tokens")
        + number(row, "cache_read_tokens")
        + number(row, "cache_creation_tokens")
}

/// The median over the sessions with input of their share read from the cache, as a whole percent rounded half up;
/// empty when no session has input (REP-12). The median of the ratios, not the ratio of the medians.
#[allow(clippy::cast_precision_loss)] // counts far below 2^52
fn cache_read_share(sessions: &[&Row]) -> String {
    let mut shares: Vec<f64> = sessions
        .iter()
        .filter_map(|r| match input_sum(r) {
            0 => None,
            sum => Some(number(r, "cache_read_tokens") as f64 * 100.0 / sum as f64),
        })
        .collect();
    shares.sort_by(f64::total_cmp);
    let n = shares.len();
    let middle = match n {
        0 => return String::new(),
        _ if n % 2 == 1 => shares[n / 2],
        _ => f64::midpoint(shares[n / 2 - 1], shares[n / 2]),
    };
    format!("{}%", (middle + 0.5).floor())
}

/// The `main_context_peak` of the sessions that have one (REP-9, REP-10): an archived row written before the column
/// is left out rather than counted as 0.
fn main_context_peaks(sessions: &[&Row]) -> Vec<u64> {
    filled(sessions, "main_context_peak")
}

/// The sessions that set an advisor (REP-11): those whose `advisor_calls` is not empty.
fn advisor_sessions(sessions: &[&Row]) -> usize {
    filled(sessions, "advisor_calls").len()
}

/// The values of `column` in the sessions whose cell is not empty.
fn filled(sessions: &[&Row], column: &str) -> Vec<u64> {
    sessions
        .iter()
        .filter_map(|r| value(r, column).parse().ok())
        .collect()
}

/// The median of `values`, empty when there is none.
fn median_or_empty(values: Vec<u64>) -> String {
    if values.is_empty() {
        String::new()
    } else {
        median(values).to_string()
    }
}

/// The `name=n;name=n` pairs of `column` summed over `sessions`, as `name=n, …`, most frequent first, then by name.
fn summed_pairs(sessions: &[&Row], column: &str) -> String {
    let mut counts: BTreeMap<&str, u64> = BTreeMap::new();
    for row in sessions {
        for (name, n) in value(row, column)
            .split(';')
            .filter_map(|p| p.split_once('='))
        {
            *counts.entry(name).or_default() += n.parse().unwrap_or(0);
        }
    }
    sorted_pairs(counts)
}

/// The sum of a column over `rows`, `None` when there is no row or one value is empty or not a number.
fn sum(rows: &[&Row], column: &str) -> Option<u64> {
    if rows.is_empty() {
        return None;
    }
    rows.iter()
        .map(|r| value(r, column).parse::<u64>().ok())
        .sum()
}

/// `a / b` with two decimals, empty when a term is missing or `b` is 0.
#[allow(clippy::cast_precision_loss)] // counts far below 2^52
fn ratio(a: Option<u64>, b: Option<u64>) -> String {
    match (a, b) {
        (Some(a), Some(b)) if b > 0 => format!("{:.2}", a as f64 / b as f64),
        _ => String::new(),
    }
}

/// `a / b` rounded half up to the unit, empty when a term is missing or `b` is 0.
fn per_unit(a: Option<u64>, b: Option<u64>) -> String {
    match (a, b) {
        (Some(a), Some(b)) if b > 0 => ((2 * a + b) / (2 * b)).to_string(),
        _ => String::new(),
    }
}

/// The rows of `inputs` whose `revision` belongs to `group`.
fn of_group<'a>(inputs: impl IntoIterator<Item = &'a Row>, group: &Group) -> Vec<&'a Row> {
    inputs
        .into_iter()
        .filter(|r| group.revisions.contains(value(r, "revision")))
        .collect()
}

/// The quality rows of `group`: those of its revisions, or, by setup, those of its setup over every non-empty revision,
/// as its sessions (REP-16); a `mixed` row matches no setup.
fn quality_of_group<'a>(inputs: impl IntoIterator<Item = &'a Row>, group: &Group) -> Vec<&'a Row> {
    if group.by_setup {
        inputs
            .into_iter()
            .filter(|r| !value(r, "revision").is_empty() && value(r, "setup") == group.name)
            .collect()
    } else {
        of_group(inputs, group)
    }
}

/// A ratio of the quality table (REP-6): its column, the summed columns it divides, how it divides them (`ratio` to two
/// decimals, `per_unit` to the unit), and the summed columns that must reach `--min-issues` for a verdict on it
/// (VRD-11, VRD-12).
struct QualityRatio {
    metric: &'static str,
    numerator: &'static str,
    divisor: &'static str,
    divide: fn(Option<u64>, Option<u64>) -> String,
    floored: &'static [&'static str],
}

const QUALITY_RATIOS: [QualityRatio; 5] = [
    QualityRatio {
        metric: "fixes_after_per_issue",
        numerator: "fixes_after",
        divisor: "issues",
        divide: ratio,
        floored: &["issues"],
    },
    QualityRatio {
        metric: "findings_per_review",
        numerator: "review_findings",
        divisor: "reviews",
        divide: ratio,
        floored: &["issues", "reviews"],
    },
    QualityRatio {
        metric: "doc_lines_per_spec_line",
        numerator: "doc_lines",
        divisor: "spec_lines",
        divide: ratio,
        floored: &["issues"],
    },
    QualityRatio {
        metric: "input_tokens_per_issue",
        numerator: "input_tokens",
        divisor: "token_issues",
        divide: per_unit,
        floored: &["token_issues"],
    },
    QualityRatio {
        metric: "output_tokens_per_issue",
        numerator: "output_tokens",
        divisor: "token_issues",
        divide: per_unit,
        floored: &["token_issues"],
    },
];

fn quality_ratio(metric: &str) -> Option<&'static QualityRatio> {
    QUALITY_RATIOS.iter().find(|r| r.metric == metric)
}

fn quality(groups: &[Group], files: &[Vec<Row>]) -> Table {
    let summed = [
        "issues",
        "deliverables",
        "commits",
        "fixes_in_issue",
        "fixes_after",
        "reverts_after",
        "reviews",
        "review_findings",
        "traced",
        "traced_without_red",
        "spec_lines",
        "doc_lines",
        "sessions",
        "failed_checks",
        "validations",
        "other_messages",
        "input_tokens",
        "cache_read_tokens",
        "output_tokens",
        "token_issues",
    ];
    let mut columns: Vec<&str> = summed
        .iter()
        .map(|c| {
            if *c == "sessions" {
                "issue_sessions"
            } else {
                c
            }
        })
        .collect();
    columns.extend(QUALITY_RATIOS.iter().map(|r| r.metric));
    table("Quality", &columns, groups, |g| {
        let rows = quality_of_group(files.iter().flatten(), g);
        let total = |c: &str| sum(&rows, c);
        let mut cells: Vec<String> = summed
            .iter()
            .map(|c| total(c).map(|n| n.to_string()).unwrap_or_default())
            .collect();
        cells.extend(
            QUALITY_RATIOS
                .iter()
                .map(|r| (r.divide)(total(r.numerator), total(r.divisor))),
        );
        vec![cells]
    })
}

fn rules(groups: &[Group], breach_rows: &[Row]) -> Table {
    let columns = [
        "rule",
        "applied",
        "breached",
        "breaches",
        "blocked",
        "breached_share",
        "bytes",
        "bytes_per_session",
    ];
    table("Rules", &columns, groups, |g| {
        let mut by_rule: BTreeMap<&str, Vec<&Row>> = BTreeMap::new();
        for row in of_group(breach_rows, g) {
            by_rule.entry(value(row, "rule")).or_default().push(row);
        }
        if by_rule.is_empty() {
            return vec![vec![String::new(); columns.len()]];
        }
        by_rule
            .into_iter()
            .map(|(rule, rows)| {
                let total = |c: &str| rows.iter().map(|r| number(r, c)).sum::<u64>();
                let bytes = sum(&rows, "bytes");
                let (applied, breached) = (total("applied"), total("breached"));
                let share = if applied > 0 {
                    format!("{}%", (breached * 200 + applied).div_euclid(applied * 2))
                } else {
                    String::new()
                };
                vec![
                    rule.to_string(),
                    applied.to_string(),
                    breached.to_string(),
                    total("breaches").to_string(),
                    total("blocked").to_string(),
                    share,
                    bytes.map(|b| b.to_string()).unwrap_or_default(),
                    per_unit(bytes, Some(applied)),
                ]
            })
            .collect()
    })
}

fn skills(groups: &[Group], context: Option<&[Row]>) -> Table {
    let mut columns = vec!["calls", "hook_refusals"];
    if context.is_some() {
        columns.push("unused");
    }
    table("Skills", &columns, groups, |g| {
        let mut cells = vec![
            summed_pairs(&g.sessions, "skills"),
            summed_pairs(&g.sessions, "hook_refusals"),
        ];
        if let Some(context) = context {
            let called: HashSet<&str> = g
                .sessions
                .iter()
                .flat_map(|r| value(r, "skills").split(';'))
                .filter_map(|p| p.split_once('='))
                .map(|(n, _)| n.rsplit(':').next().unwrap_or(n))
                .collect();
            let shipped: std::collections::BTreeSet<&str> = of_group(context, g)
                .iter()
                .flat_map(|r| value(r, "skills").split(','))
                .filter(|s| !s.is_empty())
                .collect();
            cells.push(
                shipped
                    .into_iter()
                    .filter(|s| !called.contains(s))
                    .collect::<Vec<_>>()
                    .join(", "),
            );
        }
        vec![cells]
    })
}

fn fixed_context(groups: &[Group], context: &[Row]) -> Table {
    let bytes = [
        "claude_md_bytes",
        "agents_md_bytes",
        "skill_description_bytes",
    ];
    let mut columns = bytes.to_vec();
    columns.push("total_bytes");
    table("Fixed context", &columns, groups, |g| {
        let rows: Vec<&Row> = of_group(context, g)
            .into_iter()
            .filter(|r| bytes.iter().all(|c| value(r, c).parse::<u64>().is_ok()))
            .collect();
        let n = rows.len() as u64;
        let mean =
            |f: &dyn Fn(&Row) -> u64| per_unit(Some(rows.iter().map(|r| f(r)).sum()), Some(n));
        let mut cells: Vec<String> = bytes
            .iter()
            .map(|c| mean(&|r: &Row| number(r, c)))
            .collect();
        cells.push(mean(&|r: &Row| bytes.iter().map(|c| number(r, c)).sum()));
        vec![cells]
    })
}

/// The value of `metric` for group `g` in the table VRD-1 takes it from, as printed and as a number
/// (a percent, as `breached_share` of the rule's row or `cache_read_share`, with its `%` dropped); `None` when that
/// table is not printed or the cell is not a finite number.
fn metric_value(tables: &[Table], g: usize, metric: &str) -> Option<(String, f64)> {
    let (title, column, key) = match metric {
        _ if quality_ratio(metric).is_some() => ("Quality", metric, None),
        "total_bytes" => ("Fixed context", metric, None),
        _ => match (
            metric.strip_prefix(RULE_METRIC),
            metric.strip_prefix(WASTE_METRIC),
        ) {
            (Some(rule), _) => ("Rules", "breached_share", Some(("rule", rule))),
            (_, Some(rule)) => ("Rules", "bytes_per_session", Some(("rule", rule))),
            _ => ("Efficiency", metric, None),
        },
    };
    let cell = tables
        .iter()
        .find(|t| t.title == title)?
        .cell(g, column, key)?
        .trim_end_matches('%');
    let number = cell.parse::<f64>().ok().filter(|n| n.is_finite())?;
    Some((cell.to_string(), number))
}

/// The most frequent name of `column` over the sessions of a group, the first by name among equals.
fn dominant<'a>(group: &Group<'a>, column: &str) -> Option<&'a str> {
    name_counts(&group.sessions, column)
        .into_iter()
        .max_by(|a, b| a.1.cmp(&b.1).then(b.0.cmp(a.0)))
        .map(|(name, _)| name)
}

/// The metrics read from the sessions that set an advisor (REP-11).
const ADVISOR_METRICS: [&str; 3] = [
    "advisor_calls",
    "advisor_input_tokens",
    "advisor_output_tokens",
];

/// The floors of the verdicts: the sessions of a group (VRD-9, VRD-10) and its issues (VRD-11, VRD-12).
#[derive(Clone, Copy)]
struct Floors {
    sessions: usize,
    issues: usize,
}

/// Whether group `g` holds fewer values of `metric` than its floor: sessions with a value (VRD-9, VRD-10), as only
/// `main_context_peak` and the advisor columns can be missing from a session row, or issues (and reviews) in the
/// quality table for one of its ratios (VRD-11, VRD-12).
fn too_few_values(
    tables: &[Table],
    groups: &[Group],
    g: usize,
    metric: &str,
    floors: Floors,
) -> bool {
    let too_few = |column: &str| {
        tables
            .iter()
            .find(|t| t.title == "Quality")
            .and_then(|t| t.cell(g, column, None))
            .and_then(|c| c.parse::<usize>().ok())
            .is_none_or(|n| n < floors.issues)
    };
    let group = &groups[g];
    if let Some(ratio) = quality_ratio(metric) {
        return ratio.floored.iter().any(|c| too_few(c));
    }
    match metric {
        "main_context_peak" => main_context_peaks(&group.sessions).len() < floors.sessions,
        _ if ADVISOR_METRICS.contains(&metric) => {
            advisor_sessions(&group.sessions) < floors.sessions
        }
        _ => false,
    }
}

/// The verdict of each expectation of `changes` (VRD-5, VRD-6, VRD-9, VRD-10, VRD-11, VRD-12), from the groups before
/// and after its change.
fn verdicts(groups: &[Group], tables: &[Table], changes: &[Row], floors: Floors) {
    // Per row: the revisions holding its change, and the index of its after group.
    let placed: Vec<(HashSet<&str>, Option<usize>)> = changes
        .iter()
        .map(|row| {
            let held: HashSet<&str> = value(row, "revisions")
                .split(',')
                .filter(|r| !r.is_empty())
                .collect();
            let after = groups
                .iter()
                .position(|g| g.revisions.iter().any(|r| held.contains(r)));
            (held, after)
        })
        .collect();
    let columns = [
        "change",
        "metric",
        "expected",
        "before",
        "after",
        "before_value",
        "after_value",
        "verdict",
    ];
    println!("## Verdicts\n");
    println!("| {} |", columns.join(" | "));
    println!("|{}", "---|".repeat(columns.len()));
    for (row, (held, after_index)) in changes.iter().zip(&placed) {
        let change = value(row, "change");
        let (metric, direction) = (value(row, "metric"), value(row, "direction"));
        let before_index = after_index.and_then(|i| i.checked_sub(1));
        let name = |i: Option<usize>| i.map(|i| groups[i].name.clone()).unwrap_or_default();
        let at = |i: Option<usize>| i.and_then(|i| metric_value(tables, i, metric));
        let (before_value, after_value) = (at(before_index), at(*after_index));
        let verdict = match (before_index, after_index, &before_value, &after_value) {
            (Some(b), Some(a), Some((_, bv)), Some((_, av))) => {
                let (before, after) = (&groups[b], &groups[*a]);
                // an advisor metric also moves with the advisor set (VRD-10)
                let advisor = ADVISOR_METRICS
                    .contains(&metric)
                    .then_some("advisor_models");
                let mix_changed = ["models", "versions", "project"]
                    .into_iter()
                    .chain(advisor)
                    .any(|c| dominant(before, c) != dominant(after, c));
                let other_change = changes
                    .iter()
                    .zip(&placed)
                    .any(|(other, (_, i))| value(other, "change") != change && i == after_index);
                // Only the last group can be too few in sessions, and the before group comes earlier; either can
                // be too few in values of the metric.
                if after.too_few
                    || too_few_values(tables, groups, b, metric, floors)
                    || too_few_values(tables, groups, *a, metric, floors)
                    || after.revisions.iter().any(|r| !held.contains(r))
                    || mix_changed
                    || other_change
                {
                    "confounded"
                } else {
                    moved(*bv, *av, direction)
                }
            }
            _ => "no data",
        };
        let short_change: String = change.chars().take(7).collect();
        println!(
            "| {short_change} | {metric} | {direction} | {} | {} | {} | {} | {verdict} |",
            name(before_index),
            name(*after_index),
            before_value.map(|(printed, _)| printed).unwrap_or_default(),
            after_value.map(|(printed, _)| printed).unwrap_or_default(),
        );
    }
    println!();
}

struct Options {
    sessions: Option<String>,
    breaches: Option<String>,
    quality: Vec<String>,
    context: Option<String>,
    changes: Option<String>,
    min_sessions: usize,
    min_issues: usize,
    by_setup: bool,
    anonymous: bool,
    last: Option<usize>,
}

/// The whole number from 1 to 1000 that follows the option `arg` (a floor or `--last`).
fn floor(args: &mut impl Iterator<Item = String>, arg: &str) -> Result<usize, String> {
    args.next()
        .and_then(|n| n.parse().ok())
        .filter(|n| (1..=1000).contains(n))
        .ok_or_else(|| format!("{arg} needs a whole number from 1 to 1000"))
}

fn options(args: &[String]) -> Result<Options, String> {
    let mut args = args.iter().cloned();
    let mut options = Options {
        sessions: None,
        breaches: None,
        quality: Vec::new(),
        context: None,
        changes: None,
        min_sessions: 10,
        min_issues: 5,
        by_setup: false,
        anonymous: false,
        last: None,
    };
    while let Some(arg) = args.next() {
        let mut file = || args.next().ok_or(format!("{arg} needs a file"));
        match arg.as_str() {
            "--sessions" => options.sessions = Some(file()?),
            "--breaches" => options.breaches = Some(file()?),
            "--quality" => options.quality.push(file()?),
            "--context" => options.context = Some(file()?),
            "--changes" => options.changes = Some(file()?),
            "--min-sessions" => options.min_sessions = floor(&mut args, &arg)?,
            "--min-issues" => options.min_issues = floor(&mut args, &arg)?,
            "--by" => {
                options.by_setup = match args.next().as_deref() {
                    Some("revision") => false,
                    Some("setup") => true,
                    _ => return Err("--by needs revision or setup".to_string()),
                }
            }
            "--anonymous" => options.anonymous = true,
            "--last" => options.last = Some(floor(&mut args, &arg)?),
            other => return Err(format!("unknown argument {other}")),
        }
    }
    if options.by_setup && options.last.is_some() {
        return Err(
            "--last needs groups of revisions: the setups have no order in time".to_string(),
        );
    }
    Ok(options)
}

pub(super) fn main(args: &[String]) -> u8 {
    let fail = |e: String| -> ! {
        eprintln!("{e}\n{USAGE}");
        std::process::exit(2);
    };
    let options = options(args).unwrap_or_else(|e| fail(e));
    let sessions = options
        .sessions
        .as_deref()
        .ok_or("--sessions is required".to_string())
        .and_then(|f| read(f, SESSIONS))
        .unwrap_or_else(|e| fail(e));
    let breaches = options
        .breaches
        .as_deref()
        .map(|f| read(f, BREACHES))
        .transpose()
        .unwrap_or_else(|e| fail(e));
    let quality_files: Vec<Vec<Row>> = options
        .quality
        .iter()
        .map(|f| read(f, QUALITY))
        .collect::<Result<_, _>>()
        .unwrap_or_else(|e| fail(e));
    let context = options
        .context
        .as_deref()
        .map(|f| read(f, CONTEXT))
        .transpose()
        .unwrap_or_else(|e| fail(e));
    let changes = options
        .changes
        .as_deref()
        .map(|f| read(f, CHANGES))
        .transpose()
        .unwrap_or_else(|e| fail(e));
    let mut groups = if options.by_setup {
        setup_groups(&sessions, options.min_sessions)
    } else {
        groups(&sessions, options.min_sessions)
    };
    if let Some(last) = options.last {
        let shown_from = groups.len().saturating_sub(last);
        for group in &mut groups[..shown_from] {
            group.hidden = true;
        }
    }
    let mut tables = vec![efficiency(&groups, options.anonymous)];
    if !quality_files.is_empty() {
        tables.push(quality(&groups, &quality_files));
    }
    if options.by_setup {
        // The rules, the fixed context and the verdicts are kept per revision: no setup reads them (REP-16).
        skills(&groups, None);
        return 0;
    }
    if let Some(breaches) = &breaches {
        tables.push(rules(&groups, breaches));
    }
    skills(&groups, context.as_deref());
    if let Some(context) = &context {
        tables.push(fixed_context(&groups, context));
    }
    if let Some(changes) = &changes {
        verdicts(
            &groups,
            &tables,
            changes,
            Floors {
                sessions: options.min_sessions,
                issues: options.min_issues,
            },
        );
    }
    0
}

/// The verdict of a value that went from `before` to `after` when the change expected it to go `direction` (VRD-6).
#[allow(clippy::float_cmp)] // `-0.0` equals `0.0`, as VRD-6 reads two printed values; `total_cmp` would order them
fn moved(before: f64, after: f64, direction: &str) -> &'static str {
    if after == before {
        "no observed effect"
    } else if (after > before) == (direction == "up") {
        "helped"
    } else {
        "hurt"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // spec: VRD-1 (internal: the two lists are private to their tools)
    #[test]
    fn every_quality_ratio_is_a_metric_of_an_expectation() {
        for r in &QUALITY_RATIOS {
            assert!(
                super::super::cardinal_changes::METRICS.contains(&r.metric),
                "{}",
                r.metric
            );
        }
    }

    // spec: VRD-6 (internal: the tables of the binary never print a value that is not finite)
    #[test]
    fn a_value_that_is_not_finite_is_missing() {
        let table = |cell: &str| Table {
            title: "Efficiency",
            columns: vec!["turns".to_string()],
            rows: vec![vec![vec![cell.to_string()]]],
        };
        let at = |cell: &str| metric_value(&[table(cell)], 0, "turns");
        assert_eq!(at("NaN"), None);
        assert_eq!(at("inf"), None);
        assert_eq!(at("12.50%"), Some(("12.50".to_string(), 12.5)));
    }

    // spec: VRD-6 (internal: a metric of counts never prints `-0.00`, the binary cannot reach this edge)
    #[test]
    fn equal_values_are_no_observed_effect_whatever_their_sign() {
        assert_eq!(moved(0.0, -0.0, "up"), "no observed effect");
        assert_eq!(moved(-0.0, 0.0, "down"), "no observed effect");
        assert_eq!(moved(1.0, 2.0, "up"), "helped");
        assert_eq!(moved(1.0, 2.0, "down"), "hurt");
    }
}

---
id: NXT
status: accepted
adrs: [0019, 0021]
---

# Order of the open issues

## Intent

`next-issue` tells the agent and the user which open issue comes next and which ones can start now. Each issue file
declares its priority and the issues it depends on, and the order is derived from them and from the milestone order
of the roadmap, so a branch that creates, prioritizes or closes an issue edits only its own file and never conflicts
with another branch over a shared list (ADR 0019, ADR 0021).

## Requirements

- **NXT-1** (deterministic) `next-issue` must order the open issues (the `*.md` files of `issues/`) that are ready (NXT-7) by the milestone of each (NXT-10), then by the `priority` of its front matter (the lines between a first line `---` and the next `---`, a value in quotes read without them), `high` before `medium` before `low`, then the oldest first (the date of the first commit that added the file under `issues/`, a file git does not track or outside a git repository after the tracked ones), then by stem, and print the first free one (PAR-4) with its title and the `why` of its front matter.
- **NXT-2** (deterministic) An open issue whose front matter has no `priority`, or a `priority` other than `high`, `medium` and `low`, must be ordered after the `low` ones of its milestone, reported on stderr as `issue without a priority: <stem>` with exit code 1.
- **NXT-3** (deterministic) withdrawn: issues of the same milestone and priority are ordered by the tie-breaker of NXT-1, not reported.
- **NXT-4** (deterministic) withdrawn: the format 4 `issues/queue.md` is no longer read; every consumer ranks its issue files.
- **NXT-5** (deterministic) withdrawn: `next-issue --convert` is gone with the format 4 queue it converted.
- **NXT-6** (deterministic) withdrawn: no queue is left to hide a rank.
- **NXT-7** (deterministic) An open issue is ready when every stem of the `depends_on` of its front matter (`depends_on: [<stem>, <stem>...]` on one line, `[]` or no key for none) is an issue of `issues/done/`, else blocked: `next-issue` must never offer a blocked issue.
- **NXT-8** (deterministic) A stem of `depends_on` that is neither an open issue nor an issue of `issues/done/`, or a `depends_on` that is not a list of stems, must be reported on stderr as `unknown dependency of <stem>: <dependency>` or `invalid depends_on: <stem>` with exit code 1, the issue counting as blocked by it.
- **NXT-9** (deterministic) Open issues whose dependencies form a cycle must be reported on stderr once per group of issues that depend on each other as `dependency cycle: <stem>, <stem>...` (its stems sorted) with exit code 1, the issues of the cycle staying blocked.
- **NXT-10** (deterministic) The milestone of an issue is the first non-empty line of its `# Milestone` section, and its order the position of the `## <name>` heading outside fenced code blocks and HTML comments of the same name in the `roadmap` documents of `.claude/cardinal.toml` (of the parent of the `issues` directory), read in the listed order: `next-issue` must order the issues of an earlier milestone first, and those with no milestone after those with one.
- **NXT-11** (deterministic) `next-issue` must list each blocked issue after the ready ones, in the order of NXT-1, on a line `Blocked: <stem> (by <stem>, <stem>...)` naming the stems that are not done.
- **NXT-12** (deterministic) When the free issues are all blocked, `next-issue` must print `No ready issue: every free one waits on another.` instead of an issue.
- **NXT-13** (deterministic) A milestone that no roadmap heading names must be reported on stderr as `unknown milestone of <stem>: <name>` with exit code 1, the issue ordered as one with no milestone.
- **NXT-14** (deterministic) With no `roadmap` key, or roadmap documents that cannot be read, `next-issue` must order the ready issues by priority then tie-breaker, with no milestone report.

## Scenarios

- (NXT-1) Given `b.md` of priority `high` and `a.md` of priority `low`, both with no milestone, when `next-issue` runs, then it prints `b` as next.
- (NXT-1) Given `a.md` and `b.md` of priority `medium`, committed in that order, when `next-issue` runs, then it prints `a` as next.
- (NXT-2) Given `a.md` of priority `low` and `c.md` with no front matter, when `next-issue` runs, then it prints `a` as next, lists `c` after it, reports `issue without a priority: c` and exits 1.
- (NXT-2) Given `c.md` of priority `urgent`, when `next-issue` runs, then it reports `issue without a priority: c` and exits 1.
- (NXT-7) Given `a.md` of priority `high` with `depends_on: [b]` and `b.md` of priority `low`, when `next-issue` runs, then it prints `b` as next and `Blocked: a (by b)` (NXT-11); once `b.md` is in `issues/done/`, it prints `a` as next.
- (NXT-12) Given only `a.md` with `depends_on: [b]` and `b.md` with no `depends_on`, `b` taken by a worktree, when `next-issue` runs, then it prints `No ready issue: every free one waits on another.` and `Blocked: a (by b)`.
- (NXT-8) Given `a.md` with `depends_on: [gone]` and no `gone.md` in `issues/` nor `issues/done/`, when `next-issue` runs, then it reports `unknown dependency of a: gone`, lists `Blocked: a (by gone)` and exits 1.
- (NXT-8) Given `a.md` with `depends_on: b`, when `next-issue` runs, then it reports `invalid depends_on: a` and exits 1.
- (NXT-9) Given `a.md` with `depends_on: [b]` and `b.md` with `depends_on: [a]`, when `next-issue` runs, then it reports `dependency cycle: a, b` once, lists both as blocked and exits 1.
- (NXT-10) Given a roadmap with the headings `## One` then `## Two`, `a.md` of priority `high` in milestone `Two` and `b.md` of priority `low` in milestone `One`, when `next-issue` runs, then it prints `b` as next.
- (NXT-10) Given a roadmap with an HTML comment holding `## Two`, then the headings `## One` and `## Two`, `a.md` of priority `high` in milestone `Two` and `b.md` of priority `low` in milestone `One`, when `next-issue` runs, then it prints `b` as next.
- (NXT-13) Given `a.md` in milestone `Gone` that no roadmap heading names, when `next-issue` runs, then it reports `unknown milestone of a: Gone` and exits 1.
- (NXT-14) Given no `.claude/cardinal.toml`, `a.md` of priority `low` in milestone `One` and `b.md` of priority `high`, when `next-issue` runs, then it prints `b` as next and exits 0.

## Non-goals

- Reading the `rank` of a format 5 issue: an issue without a priority is reported (NXT-2) until `skill:prioritize-issues` gives it one.
- Checking that the issues a roadmap milestone lists are those that name it: the `# Milestone` section of the issue is the source of the order.
- Dependencies on anything but an issue (a measure, a number of sessions): the `why` and the Hints state them.
- Validating the priority and the dependencies of a new issue file: `check-issue` does it (`specs/check-issue.md`).

---
id: CLR
status: accepted
adrs: []
---

# Spec clarity checks

## Intent

`check_specs.rs` makes the ambiguities of a spec visible before the user validates it: an open question the model
marks instead of guessing, and the wording of a requirement that leaves its obligation vague. A marker blocks an
`accepted` spec; the wording checks only warn a `draft`, since a word list has false positives.

## Requirements

- **CLR-1** (deterministic) The checker must report a problem, and so exit 1, for each `[NEEDS CLARIFICATION:` marker of an `accepted` spec, naming the file and the line.
- **CLR-2** (deterministic) For each `[NEEDS CLARIFICATION: <question>]` marker of a `draft` spec, the checker must print a warning to stderr naming the file, the line and the question, without failing.
- **CLR-3** (deterministic) For each requirement of a `draft` spec whose statement holds one of the words `should`, `may`, `etc.`, `appropriate`, `fast` (whole word, any case), the checker must print a warning to stderr naming the requirement and the word, without failing.
- **CLR-4** (deterministic) For each requirement of a `draft` spec whose statement does not hold exactly one `must` (whole word, any case; `must not` counts as one), the checker must print a warning to stderr naming the requirement and the count, without failing.
- **CLR-5** (deterministic) The marker search of CLR-1 and CLR-2 and the word checks of CLR-3 and CLR-4 must ignore the text inside inline code spans (between backticks) and inside fenced code blocks.
- **CLR-6** (deterministic) The word checks of CLR-3 and CLR-4 must ignore the text inside double quotes and skip a withdrawn requirement.
- **CLR-7** (deterministic) The checker must not print a wording warning (CLR-3, CLR-4) for an `accepted` spec.

## Scenarios

- (CLR-1) Given an `accepted` spec whose Scenarios hold `[NEEDS CLARIFICATION: which exit code?]`, when the checker runs, then it exits 1 with a line naming the spec and that line.
- (CLR-4) Given a `draft` requirement "The hook must exit 0 and must not print", when the checker runs, then it warns that the requirement holds 2 `must`.
- (CLR-5) Given a `draft` requirement citing `` `[NEEDS CLARIFICATION: x]` `` or `` `may_fail` ``, when the checker runs, then neither is reported.
- (CLR-6) Given a `draft` requirement "The hook must print \"you may retry\"", when the checker runs, then `may` is not reported.
- (CLR-5) Given a fenced block opened by four backticks that holds a line of three, when the checker runs, then the
  block ends only at the next line of four or more backticks.
- (CLR-6) Given a `draft` requirement with an unclosed quote (an inch mark), when the checker runs, then the text after
  it is still checked; `shouldn't` holds `should` and `mustn't` counts as one `must`.

## Non-goals

- The format and traceability checks the checker already runs (front matter, sections, ids, citations): out of this
  spec, tested by `test_check_specs.rs`.
- A marker spread over several lines.
- Judging whether a requirement is complete or observable: that is the checklist of `skill:write-spec`, read by the
  model, not a check.

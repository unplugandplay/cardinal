---
priority: medium
why: 'the only open issue'
---
# Count the frequency of each word

# Goal
A caller can count the words of a text but not tell which words come most often. The outcome: `word_frequencies`
in `src/lib.rs`, as `specs/text-statistics.md` requires.

# Findings
* `word_count` exists and is tested (TXT-1); TXT-2 and TXT-3 are traced by no test yet.

# Approach
One function next to `word_count`, with its unit tests.

# Resources
* `specs/text-statistics.md`.

# Tracking

Plan: validated

## Understanding
Add `word_frequencies` (TXT-2, TXT-3) next to `word_count`; nothing else changes.

## Plan
1. [ ] `word_frequencies`: counts the lower-cased alphanumeric words and sorts them; verified by unit tests in
   `src/lib.rs`; makes true TXT-2, TXT-3.

## Decisions
- Mode: autonomous
- Base commit: BASE

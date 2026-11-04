---
id: TXT
status: accepted
adrs: []
---

# Text statistics

## Intent

`src/lib.rs` gives the statistics of a text that a caller prints: how many words it has and which come most often.

## Requirements

- **TXT-1** (deterministic) `word_count(text)` must return the number of whitespace-separated words of `text`.
- **TXT-2** (deterministic) `word_frequencies(text)` must return each distinct word of `text` with the number of its
  occurrences, as `Vec<(String, usize)>`, a word being a maximal run of alphanumeric characters, lower-cased (so
  `Hello,` and `hello` are one word and `don't` is the two words `don` and `t`).
- **TXT-3** (deterministic) The pairs of `word_frequencies` must be sorted by count, highest first, then by word in
  ascending order; a text with no word gives an empty vector.

## Scenarios

- (TXT-1) Given ` one  two\nthree `, when the words are counted, then the count is 3.
- (TXT-2) Given `The cat; the CAT!`, when the frequencies are read, then they are `[("cat", 2), ("the", 2)]`.
- (TXT-3) Given `b a b c`, when the frequencies are read, then they are `[("b", 2), ("a", 1), ("c", 1)]`.

## Non-goals

- Unicode word segmentation beyond `char::is_alphanumeric`.

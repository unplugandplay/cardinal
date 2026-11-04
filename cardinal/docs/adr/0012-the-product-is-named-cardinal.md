# 0012. The product is named Cardinal

Status: Accepted
Date: 2026-10-28

## Context
"harness" named both the product (this repository, its binary, its files) and the concept it implements, an agent
harness. A sentence such as "the harness revision" or "fix it in the harness" could mean either, and the product
could not be named apart from any other agent harness. The repository is already named `cardinal`.

## Decision
The product is Cardinal: "Cardinal, the shared agent harness of the user's projects". "harness" keeps its generic
meaning (an agent harness). The rename goes in steps, each its own issue, so a consumer never breaks:

1. The prose of the docs and the shared texts (issue `name_the_product_cardinal`).
2. The binary and crate `harness` become `cardinal`, with a transition for the consumers that have the old binary.
3. The consumer files (`.claude/harness.toml`, `.claude/harness.lock`, `harness.just`), migrated by `sync`.
4. The tools, skill, spec, commit scope, workflow and recipe still named after the product.

Until a step lands, the text names its commands, files and paths as they are (`harness sync`, `harness.toml`).

## Consequences
The docs say "Cardinal" next to commands, files and tools still named after `harness` until steps 2 to 4 land. A
name that means the concept (an agent harness) may keep "harness" at step 4.

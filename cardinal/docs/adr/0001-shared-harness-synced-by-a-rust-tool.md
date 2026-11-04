# 0001. Shared harness, synchronised by a Rust tool

Status: Accepted
Date: 2026-10-24

## Context
Several projects use the same agent harness (skills, hooks, scripts, recipes). Improvements made in one had to be
ported by hand to the others, adapting paths and wording each time. A comparison of two projects showed that the
code is almost identical: they differ by a few constants and by sentences of the skills.

## Decision
One repository holds the harness; each project receives byte-identical copies, tracked by a lock file of hashes.
What differs becomes a value in the project's `.claude/harness.toml`, read by the scripts and named by the
skills. A small Rust tool (`harness sync`, `check`, `init`) does the copy. A local edit of a managed file is a
conflict, never merged: it is fixed in this repository.

Rejected: a Python templating tool (`copier`), because the user does not want Python as a dependency; templated
files with a three-way merge, more machinery than a few values justify; git submodules or subtrees, which
cannot express per-project values; a Claude Code plugin alone, which cannot carry recipes, CI or documents.

## Consequences
A project cannot tweak a shared file: it asks for a new value here. Skills must stay free of project paths. The
tool, a binary on the user's `PATH`, is the only prerequisite besides `just` and `rust-script`. This repository is
its own first consumer.

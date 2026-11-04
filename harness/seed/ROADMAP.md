# Roadmap

The planned work of the project, grouped into milestones, in the order they are meant to land: the user orders them.
A milestone becomes issues once it is shaped (`skill:create-issue`); the `priority` and `depends_on` of each issue file order them within a milestone (`just next-issue`).
States: `idea`, `shaped`, `in issues`, `done` (set by `just close-issue` when the last issue of an `in issues` one closes); `just release` adds `- Released: v<version>` to a `done` one. A milestone carries no date unless the project gives it one.

No milestone yet. Add one section per milestone, in this shape:

<!--
## <Milestone name>

- Goal: <what is true once the milestone is done>
- Why: <the problem it solves, or what it unblocks>
- State: idea
- Issues:
  - [<stem>](issues/<stem>.md)
-->

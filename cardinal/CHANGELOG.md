# Changelog

The notable changes of each version, newest first ([Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
[Semantic Versioning](https://semver.org/)). Each section is written by `just release <version>` from the commits,
then reread before the release is committed.

## [3.1.0] - 2026-11-03

### Milestones

- Lean agent loop

### Added

- count only the fixes of the lines an issue wrote
- tell the oversized results of the reviewers apart
- print only the last groups of the report
- print a review target without the issues and copies
- hand the reviews a target without the issues and copies
- fold the delegated exploration back into the main loop
- name the commit skill as the next step of an update
- send the agent back once before a whole read of a large file

### Fixed

- blame the fixes whatever the git configuration
- count only the project commits skipping the message skill
- read the commands of a commit call in their order
- run the setup report of evaluate-harness without --last
- review the uncommitted edits and untracked files too
- know the four new line types of the transcripts
- judge the update next step where the update writes
- estimate the result of a read as Claude Code returns it

## [3.0.0] - 2026-11-02

### Milestones

- Optional features
- Longer autonomy

### Breaking

- let a project turn off a feature of the harness
- ask the user before an agent writes a release

### Added

- make the ban on AI attribution a feature to turn off
- let a project turn off the lint, format and language hooks
- let a feature need another
- leave the commit format to a project with its own
- remove the native gate the source no longer manages
- let a project turn off the review gate
- keep the language rule out of the report style
- declare the features of the managed files only
- set the milestone done when its last issue closes
- let the close commit the milestone it sets done
- allow the release proposal in auto mode
- let a project turn off the issue files
- let a project turn off the specs
- let a project turn off the workspace lock
- release a project that turns off conventional-commits
- review the issues directory with the issue files off
- count the issues directory with the issue files off
- skip the validation stop rule with the issue files off
- sign the validations of an issue with the git identity
- upgrade the installed cardinal to the binary of a commit
- name cardinal upgrade in the install hint
- stage the close of an issue for a bare commit
- chain a queue of a few issues from one session

### Fixed

- hand over the worktree lock of a session with no work
- judge a changed lock holder instead of refusing it
- leave the features turned off out of update and the seeds
- remove the gate only once the lock records none
- quiet the review rules of rule-breaches with the gate off
- keep the warnings on the project's own retention
- read the roadmap of a close as the user writes it
- propose the release with cardinal run release
- stop citing issue-workflow in the review refusal
- gate the specs check on cardinal-specs.just
- find the project root of an issue checked in issues/done
- resolve the root of an issue like cardinal run does
- head the unsorted release section Changed
- tie the issues exceptions of the gates to the issue files
- sign a deliverable only once a person validated it
- upgrade the cardinal of the PATH, not the running one
- read each commit of a release with its own convention
- warn of the commits release reads without the convention
- read the roadmap milestones outside fences and comments
- close a roadmap fence and comment as Markdown does
- review the release commit and tag it with cardinal run
- refuse a done issue in check-issue with one problem
- resolve the directory of an issue once in check-issue
- refuse a dangling pre-commit.user at the gate install
- keep the hooks unchanged when the gate install fails
- never replace a hook at the gate install
- name the chained hook conflict in check
- share the chained hook conflict text of sync and check
- stage only the close of an issue, names read literally
- finish a gate install a killed sync left half done
- keep a queue of issues resumable and its diffs apart

## [2.0.0] - 2026-11-01

### Breaking

- stop reading the format 4 issues/queue.md in next-issue
- sync the format 5 and higher only
- order the issues by priority and dependencies

### Added

- review the existing instructions in audit-instructions
- count the changed lines of a deliverable with a tool
- count the staging of every file in rule-breaches
- refuse a git add of every file before a commit
- add a recipe that replays the fixture issue headless
- stop editing issues/queue.md in close-issue
- check the priority and dependencies of a new issue
- prioritize the issues instead of ranking them
- claim an issue for the session that works it
- free the claim of an issue when it is closed
- keep one Claude Code session per worktree
- claim the issue at the start of issue-workflow
- open a worktree with no issue attached
- close a worktree by its name, path or branch

### Fixed

- settle what the high review of the audit found
- let git count the untracked files of deliverable-size
- read a shell line once for the staging gate and rule
- grade a replay run on its end, commits, review and test
- keep the age of a renamed issue in next-issue
- free merged claims and keep claims to their worktree
- refuse the worktree closes that could drop the wrong work
- count a removed worktree with its repository by name
- prefer the branch match for a removed worktree

## [1.2.0] - 2026-10-31

### Milestones

- Advisor

### Added

- count the advisor calls and tokens of each session
- show the advisor columns in the efficiency table
- give a verdict on the expected advisor effects
- let the close of an issue commit without a review
- name when the issue workflow consults the advisor
- show the cache read share in the efficiency table
- sum the tokens spent on each closed issue
- report the tokens per closed issue
- leave out the issues whose tokens are unknown
- require enough issues for a per-issue verdict
- narrow the delegated exploration to wide searches
- require enough issues and reviews for a ratio verdict
- flag the skills a session should have loaded
- flag the tool results that waste tokens
- weigh the bytes of the waste rules per revision
- give the bytes per session of a waste rule a verdict
- keep the waste rules out of the hook candidates
- group the report by the setup of the sessions
- split the delivered quality by setup
- compare the model setups in the harness evaluation

### Fixed

- confound an advisor verdict when the advisor set changed
- refuse a close commit that holds more than rewrites
- measure the main context peak of an advised message
- take the iteration peak only on an advised message
- settle the advisor moments the high review found loose
- cover the verdict on the cache read share
- count a resumed message once in the tokens of an issue
- shorten the delegated search and tell it from a git log
- read the skill routing triggers more strictly
- accept the waste bytes metric for a waste rule only
- read a model advised by itself as a solo setup
- settle the setup comparison the high review found loose

## [1.1.0] - 2026-10-30

### Added

- end close-issue on the next issue to work on
- end the close report on a link to the next issue

## [1.0.0] - 2026-10-30

### Milestones

- Team use

### Breaking

- rank each issue in its own file

### Added

- read the user language from a personal file
- measure the user language from the personal file
- carry the personal configuration into issue worktrees
- order the issues by the rank of each issue file
- convert the issue queue into the ranks of the files

## [0.2.0] - 2026-10-30

### Milestones

- Plan and release

First numbered release.

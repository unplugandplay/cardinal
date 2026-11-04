# Development commands. `just` lists them; the agent harness uses the same entry points.
import 'cardinal.just'

default:
    @just --list

# Format, lint and test the project, then check the specs (when the feature `specs` is on). Add the first three, the
# project's own, quiet on success, above the last line: the agent reads all of their output.
check:
    @if [ -f cardinal-specs.just ]; then "{{just_executable()}}" check-specs; fi

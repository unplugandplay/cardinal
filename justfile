# Development commands. `just` lists them; the agent harness uses the same entry points.
import 'cardinal.just'

default:
    @just --list

# Format, lint and test the project. Add the project's own commands here, quiet on success: the agent reads their
# output. No project checks are configured yet.
check:
    @echo "no project checks configured"

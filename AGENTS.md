# AGENTS.md

## Agent skills

### Issue tracker

Issues and specs live as GitHub issues in `Jheickson/Wordle`, managed with the `gh` CLI (`gh issue create`, `gh issue view`, `gh issue list`). See `docs/agents/issue-tracker.md`.

**Issue before code.** Every behavior change starts with a GitHub issue — new functionality, bug fixes, and refactors that restructure code (module splits, dependency swaps, architectural changes). Create it before the first code change, carrying the spec and acceptance criteria for a feature, the repro for a bug, or the target design for a refactor; reference `#<n>` in the work. Typos, formatting, and docs-only edits go straight in; if a trivial fix grows into any of the above, stop and file the issue first.

**Type and tag every issue.** Every issue is created with its type (`Bug`, `Feature`, `Task`) and the relevant labels already applied — kind (`bug`, `enhancement`, `documentation`, `question`) plus the triage role (`needs-triage` first). If no existing type or label fits, ask the user before creating a new one. Details in `docs/agents/issue-tracker.md`.

**Complete issues through commit messages.** The commit that resolves an issue MUST carry a closing keyword in its body — `Closes #<n>` (`Fixes`/`Resolves` also work; GitHub closes the issue when the commit reaches `main`). Prefer this over `gh issue close` for any work that lands in a commit: the commit message is the record of completion. Post the validation/explanation with `gh issue comment <n>` (before the closing push, or on the auto-closed issue afterwards). Use manual `gh issue close` only when no commit can close the issue (e.g. it is resolved by an already-merged change or by documentation elsewhere), and state the reason in the comment.

### Commit messages

Use [Conventional Commits](https://www.conventionalcommits.org/): `<type>(<scope>): <subject>`.

- Types in use: `feat`, `fix`, `docs`, `refactor`, `test`, `chore`, `perf`, `build`, `ci`.
- Subject after the colon: lowercase, imperative mood, no trailing period.
- Scope names the area touched (`ui`, `game`, `words`, `agents`); omit for repo-wide changes.
- Put the issue-closing keyword in the commit body (`Closes #<n>`), never in the subject.
- Breaking changes add `!` after the type/scope and a `BREAKING CHANGE:` footer.

The history is the record: never leave a commit with a bare subject (`Add …`, `Fix …`).

### Triage labels

The five canonical triage roles use their default label strings: `needs-triage`, `needs-info`, `ready-for-agent`, `ready-for-human`, `wontfix`. See `docs/agents/triage-labels.md`.

### Domain docs

Single-context layout: domain vocabulary lives in a root `GLOSSARY.md` and decisions in `docs/adr/`, read before exploring and created lazily. See `docs/agents/domain.md`.

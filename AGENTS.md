# AGENTS.md

## Agent skills

### Issue tracker

Issues and specs live as GitHub issues in `Jheickson/Wordle`, managed with the `gh` CLI (`gh issue create`, `gh issue view`, `gh issue list`). See `docs/agents/issue-tracker.md`.

**Complete issues through commit messages.** The commit that resolves an issue MUST carry a closing keyword in its body — `Closes #<n>` (`Fixes`/`Resolves` also work; GitHub closes the issue when the commit reaches `main`). Prefer this over `gh issue close` for any work that lands in a commit: the commit message is the record of completion. Post the validation/explanation with `gh issue comment <n>` (before the closing push, or on the auto-closed issue afterwards). Use manual `gh issue close` only when no commit can close the issue (e.g. it is resolved by an already-merged change or by documentation elsewhere), and state the reason in the comment.

### Triage labels

The five canonical triage roles use their default label strings: `needs-triage`, `needs-info`, `ready-for-agent`, `ready-for-human`, `wontfix`. See `docs/agents/triage-labels.md`.

### Domain docs

Single-context layout: domain vocabulary lives in a root `GLOSSARY.md` and decisions in `docs/adr/`, read before exploring and created lazily. See `docs/agents/domain.md`.

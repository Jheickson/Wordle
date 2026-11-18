# AGENTS.md

## Agent skills

### Issue tracker

Issues and specs live as GitHub issues in `Jheickson/Wordle`, managed with the `gh` CLI (`gh issue create`, `gh issue view`, `gh issue list`). See `docs/agents/issue-tracker.md`.

### Triage labels

The five canonical triage roles use their default label strings: `needs-triage`, `needs-info`, `ready-for-agent`, `ready-for-human`, `wontfix`. See `docs/agents/triage-labels.md`.

### Domain docs

Single-context layout: domain vocabulary lives in a root `GLOSSARY.md` and decisions in `docs/adr/`, read before exploring and created lazily. See `docs/agents/domain.md`.

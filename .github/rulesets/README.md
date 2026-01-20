# GitHub Branch Protection (Rulesets)

This repo includes **example ruleset JSON** under `.github/rulesets/`.

GitHub Rulesets can be created via:
- GitHub UI: Settings → Rules → Rulesets
- GitHub API: create/update rulesets using the JSON files in this folder

Notes:
- The required status check contexts MUST match the check names produced by GitHub Actions.
  See `.github/ci-required-checks.md`.
- Squash-merge allowance is configured in repo settings (Settings → General → Pull Requests).

# Branching & Release Process

This repository follows a **stable main + versioned release lines + dev integration** model.

## Branches

- `main` — latest stable only. Every commit on `main` MUST be releasable.
- `release/1.0` — maintenance line for v1.0.x patch releases.
- `develop/1.1` — integration branch for v1.1 development work (may evolve, but must pass CI).
- `release/1.1.0` — stabilization branch cut from `develop/1.1` when feature-complete; only fixes and release hardening.

## Workflow

1. Create a topic branch from the appropriate base:
   - `feature/*` from `develop/1.1` for v1.1 work
   - `hotfix/*` from `release/1.0` for v1.0.x patch fixes
2. Open a PR into the target branch (`develop/1.1` or `release/1.0`).
3. CI MUST pass and reviews MUST be complete before merging.
4. Release tags are created from the corresponding `release/*` branch (or `main` after merge).

## Tags

- Stable releases: `vX.Y.Z`
- Development prereleases: `vX.Y.Z-dev.N`
- Release candidates (optional): `vX.Y.Z-rc.N`

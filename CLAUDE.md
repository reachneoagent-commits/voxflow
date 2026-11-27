<!-- Voxflow (rebrand/apply.py) -->
# Voxflow

This repository is **Voxflow**, a fork of [Handy](https://github.com/cjpais/Handy) with its own features on top. Read `docs/UPSTREAM.md` first. In short:

- Voxflow's name, bundle id, updater endpoint and signing key are guarded by `rebrand/brand.json`: after merging upstream, run `python3 rebrand/apply.py` and `--check`.
- Shared Handy packages keep their names: `handy-keys`, `handy-recorder`, `@handy-computer/models`, the blob.handy.computer model URLs.
- Never push `upstream/*` tags, and never post to cjpais/Handy.

Read @AGENTS.md

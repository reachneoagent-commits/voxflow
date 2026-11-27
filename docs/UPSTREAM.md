# Keeping up with upstream

Voxflow is a fork of [Handy](https://github.com/cjpais/Handy) by CJ Pais (MIT). It carries its own
features on top (engine supervisor, system keychain for API keys, corrections, a strict CSP and
more), so unlike the other FlowApps forks most of its changes are ordinary code, merged by hand.
What must never slip back to Handy's in a merge is Voxflow's identity, and `rebrand/apply.py`
guards that.

## The identity guard

`rebrand/apply.py` is the same script in every FlowApps fork; `rebrand/brand.json` lists what it
enforces here:

- package and crate names (`voxflow-app`, `voxflow`, `voxflow_app_lib`), description and authors;
- `productName` Voxflow and bundle id `com.voxflow.app` (changing it would lose every user's
  settings, models and history);
- the updater endpoint and minisign key (our releases, our key), with CJ's Windows signing
  command removed;
- the Linux library folder `/usr/lib/Voxflow`;
- notes added once to `CLAUDE.md` and `.gitattributes`.

`.gitattributes` keeps our `README.md`, `src-tauri/icons/` and `.github/workflows/release.yml` in
every merge.

Left alone on purpose: shared Handy packages and services (`handy-keys`, `handy-recorder`,
`@handy-computer/models`, the blob.handy.computer model URLs), "(Handy #NNNN)" issue references,
and the licence.

## Which upstream version we follow

Handy tags its releases on `main`; Voxflow follows `main`, and `rebrand/upstream.json` records the
upstream commit we last merged. Voxflow's first commit was a copy of Handy at 7bfb2e2; a
history-only merge records that commit, so git finds the right merge base.

## Merging upstream

Set the clone up once, so upstream's tags arrive as `upstream/v…` and never collide with ours:

```bash
git remote add upstream https://github.com/cjpais/Handy.git
git config remote.upstream.tagOpt --no-tags
git config --add remote.upstream.fetch '+refs/tags/*:refs/tags/upstream/*'
git config merge.ours.driver true
```

Never push the `upstream/*` tags, and never open issues or PRs on cjpais/Handy from this fork.
Then:

```bash
git fetch upstream
git checkout -b upstream-sync
git merge upstream/main
```

Resolve conflicts keeping Voxflow's features and names, then:

```bash
python3 rebrand/apply.py
python3 rebrand/apply.py --check
bun install && bun run build && bun run lint
cd src-tauri && cargo test
```

Update `rebrand/upstream.json` to the merged commit and open a PR. Merge it with a merge commit,
never a squash, so the next merge finds its base.

If `apply.py` says it can't find an edit, upstream changed that line: update the `find` text in
`brand.json` to upstream's new wording.

## Automatic check and prepare

`rebrand/upstream.py` (the same file in every FlowApps fork) and two workflows do the routine part:

- **Upstream check** runs every Monday and on demand. It keeps one `upstream-update` issue current:
  the new upstream commits, a trial merge, and whether the rebrand still applies. It closes the issue
  once we're up to date.
- **Upstream prepare** runs only by hand. It merges upstream on an `upstream-sync/<date>` branch,
  re-applies the branding, checks it, updates `rebrand/upstream.json` and opens a pull request for
  review. It needs the `UPSTREAM_PUSH_TOKEN` secret (see the workflow). If a conflict or the rebrand
  needs a person, it stops without pushing anything; then merge by hand as above.

`"conflicts"` in `rebrand/upstream.json` is `"stop"`: any conflict stops the prepare step, because this fork carries its own code. Upstream's own workflow changes are
never brought in automatically; the pull request lists them. Locally: `python3 rebrand/upstream.py
status`, `report --out FILE` or `prepare --body FILE`.

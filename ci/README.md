# ci/

The Rust gate's script, kept here so that you can run the same gate
locally.

- `rust/run.sh` is the Rust gate. `.github/workflows/rust.yml` runs it,
  and so can you.

The workflows themselves live in `.github/workflows/`. To change CI, edit
them there in a reviewed pull request: session credentials can push
workflow changes (admin `DECISIONS.md` ADR-8, as amended on 2026-09-24),
so staging a workflow here for a maintainer to promote is optional.
Sessions still cannot push tags. Releases therefore go through
`workflow_dispatch`, and a workflow that runs only on a tag push needs a
maintainer to push that tag.

Six of this repository's workflows also have a template in admin
`rollout/workflows/`: `ci.yml`, `crates-release.yml`, `deps-gate.yml`,
`notify-status.yml`, `release.yml` and `scorecard.yml`. ADR-8 as amended
says a workflow changed here is mirrored in its template, so change the
template too, in admin. Otherwise admin `scripts/verify.sh` reports the
drift, and the next `rollout/apply-workflows.sh --apply` pushes the old
text back. `clib.yml` and `clib-release.yml` are stamped (each carries a
`tabnas-clib-template` marker): they change only through admin
`tasks/clib-template/` and a re-stamp with `tasks/adopt-clib.sh`, never
by hand. The others have no template and change here alone.

## Promoted

Both of these were staged here and now run from `.github/workflows/`:

- **`rust.yml`**, the Rust gate. It checks this repository out
  into a named directory, clones the `parser` and `support` siblings
  beside it (both are unpublished path dependencies of `rs/Cargo.toml`),
  installs the MSRV toolchain pinned in `rs/Cargo.toml`, and runs
  `ci/rust/run.sh`: `cargo fmt --check`, a build of every target, the
  tests, the doctests, clippy at `-D warnings`, and a lockfile check that
  masks only the two sibling crates' recorded versions. Everything lives
  in the script so a local run and the hosted one cannot say different
  things; `ci/rust/run.sh` runs it here, and `make test-rs` is the fast
  inner loop.

- **`docs.yml`** — the prose gate: Vale over the reader-facing pages at
  the levels set in `.vale.ini`, on the file list
  `ts/scripts/gated-docs.cjs` produces. See `docs/STYLE-GUIDE.md`.
  `make prose` runs the same check locally.

# ci/

Staging area for GitHub Actions workflow changes.

This directory exists because session credentials cannot write
`.github/workflows/*` — see admin `DECISIONS.md` ADR-8. To change CI:

1. Put the intended workflow file in `workflows/`.
2. A maintainer promotes it with the admin `rollout/apply-ci-folders.sh`
   script.

## Pending

Nothing.

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

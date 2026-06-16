# Agents Guide — hoover

## What this project is

hoover is a **syntax plugin for the [tabnas](https://github.com/tabnas/parser)
parser engine**. It adds configurable, block-delimited string parsing —
what the project calls *hoovering*: vacuuming up a run of source text
(including internal spaces and newlines the lexer would otherwise split
on) between a start and end delimiter, with optional escape handling and
rule-context gating.

hoover is **grammar-agnostic**: it adds an alternate to the host
grammar's `val` rule, and its only production dependency is the engine.
Two canonical shapes drive the design:

- **triple-quoted strings** — `'''hello world'''` → `"hello world"`,
  preserving spaces and newlines.
- **end-of-line / terminated values** — an unquoted run of text up to a
  newline, `#`, `;`, or end-of-input, captured as one string.

It is a lexer-level plugin: it installs a single lex matcher
(`makeHooverMatcher`, registered as `lex.match.hoover` at order `4.5e6`,
ahead of the string and number matchers) that emits a `#HV` token, and
registers that token as an extra `val` alternate. The matching itself is
`matchStart` (rule-context + start-delimiter) then `parseToEnd` (forward
scan, escapes, value resolution) — the two functions the Go port mirrors
by name.

## Repository map

| Path | What it is |
|---|---|
| [`ts/`](ts/) | **Canonical** TypeScript/JavaScript implementation — the `@tabnas/hoover` npm package (`0.13.1`). A single plugin in [`ts/src/hoover.ts`](ts/src/hoover.ts). Imports the engine as `@tabnas/parser`; peer-depends on it (`">=2"`). |
| [`go/`](go/) | Go port — module `github.com/tabnas/hoover/go` (`const Version` in `go/hoover.go`, `0.1.7`), a single [`go/hoover.go`](go/hoover.go). Depends only on `github.com/tabnas/parser/go` (imported as `tabnas`). |
| [`ts/doc/hoover-ts.md`](ts/doc/hoover-ts.md), [`go/doc/hoover-go.md`](go/doc/hoover-go.md) | Per-runtime tutorial → how-to → reference → explanation docs. |

There are no shared `.tsv` fixtures and no grammar package: hoover's
only production dependency is the engine, and each runtime brings its
own tiny local test grammar (`val` + a parenthesised `group`). `ts/` and
`go/` each have their own `AGENTS.md` with layout and contribution notes.

## The tabnas engine dependency

Both runtimes depend on the unpublished engine as a **sibling
checkout**, the standard tabnas dev model until `tabnas/parser` publishes
tagged packages:

- TypeScript: `@tabnas/parser` is declared as a `peerDependency`
  (`">=2"`) in `ts/package.json` and mirrored as a
  `file:../../parser/ts` devDependency for local builds. `@tabnas/debug`
  (`file:../../debug/ts`) and `@tabnas/railroad` (`file:../../railroad/ts`)
  are also listed as dev-only `file:` devDependencies, but — unlike the
  grammar repos — this repo currently has **no** `debug-model` test and
  **no** generated railroad diagram, so nothing imports them. Treat them
  as latent: don't claim a `debug.model()` test or a `grammar.svg`
  exists here. The only `@tabnas` package under `ts/node_modules` is
  `parser`.
- Go: `go/go.mod` requires `github.com/tabnas/parser/go` with
  `replace github.com/tabnas/parser/go => ../../parser/go`. That is the
  module's **only** dependency.

Clone `https://github.com/tabnas/parser` as a sibling of this repo and
build the engine's TS (`cd parser/ts && npm install && npm run build`),
then work here. CI clones it (and the rest of the closure) for you (see
CI below).

## Authority and alignment rules

1. **TypeScript is canonical, and you work on it first.** Make every
   behavior change in `ts/src/hoover.ts` first, then port it to
   `go/hoover.go` in the same change. When TS and Go disagree, TS wins;
   change Go to match. The engine (tabnas) is 1-based for row/column
   tracking in both languages — keep hoover consistent with that (columns
   reset to `1` after a newline).
2. Neither runtime depends on a grammar package, so there are no shared
   JSON fixtures. Parity is kept by testing both ports against an
   **identical tiny local grammar** with **matching cases**:
   [`ts/test/minigrammar.ts`](ts/test/minigrammar.ts) and
   `go/minigrammar_test.go` define the same `val` + `group` grammar, and
   [`ts/test/hoover.test.ts`](ts/test/hoover.test.ts) / `go/hoover_test.go`
   assert the same inputs and outputs. Add a case to both in the same
   change.
3. The configuration shape is the same in both languages: `block` is an
   **ordered array** of block definitions, each with a `name`. Blocks
   are tried in array order, so order is significant and must be
   preserved (the Go port must not iterate a map — it uses `[]*Block`).
4. Once a block's start matches, the block is **committed**: failing to
   reach an end delimiter (or hitting a rejected escape) is an error
   (a bad token: `invalid_text` for an unterminated block,
   `invalid_escape` for a rejected escape), not a silent fall-through to
   the next block or matcher.
5. Hoover is a **grammar-agnostic plugin**: it extends the host
   grammar's `val` rule, so it must be registered on an instance that
   already carries a grammar defining `val` (the engine itself ships
   none). Register the dependency grammar first, then the hoover plugin.
   Hoover **fails fast** with a clear error if the `val` rule is absent
   (`tn.rule()` returns no `val`), rather than creating an empty one and
   failing confusingly later. Keep this guard in both runtimes.

## Repo-specific gotchas

- **Block option defaults are filled in `buildBlocks`**, not by the
  engine: each block gets `token: '#HV'` unless overridden,
  `allowUnknownEscape: true` and `preserveEscapeChar: false` when unset.
  A custom per-block `token` registers a distinct token but only the
  first occurrence of each token name adds a `val` alternate
  (`tokenMap`). Keep these defaults aligned with Go.
- **`start.rule` gating** (the `matchStart` rule-context check) defaults
  to parent `pair`/`elem`-style matching via `current`/`parent`
  include/exclude lists and a `state` string. `state` defaults to `'o'`
  (open); `state: ''` means *don't check the state* — and this is a known
  Go gap: Go's zero-value `State == ""` cannot mean "skip" the way TS
  does, because the Go zero value defaults to `"o"`. Documented in
  `go/AGENTS.md`; don't "fix" it without an engine change.
- **`consume`** on `start`/`end` is `null | boolean | string[]`: `null`
  (default) and `true` consume the delimiter, `false` leaves it in the
  stream, an array consumes only the listed delimiters. Mirrored in Go
  as `any` (`nil`/`bool`/`*bool`/`[]string`).
- **Value resolution** happens in `parseToEnd`: if `cfg.value.lex` is on
  and the hoovered text is a registered value keyword
  (`cfg.value.def[val]`), the token value becomes that keyword's value.
  The bare Go engine ships no `value.def` keywords (`true`/`false`/`null`)
  while the TS engine does, so the Go test grammar defines them — another
  documented Go/TS gap.

## Build & test

TypeScript (from `ts/`):

```bash
npm install          # auto-installs the @tabnas/parser peer; resolves file: siblings
npm run build        # tsc --build src test  → dist/ and dist-test/
npm test             # node --test over dist-test/*.test.js
```

Go (from `go/`):

```bash
go build ./... && go vet ./...
go test ./...        # drives the plugin through go/minigrammar_test.go
```

The TS suite runs against **compiled output** — always `npm run build`
after editing `ts/src/` or `ts/test/*.ts`.

Both the repo-root [`Makefile`](Makefile) and [`ts/Makefile`](ts/Makefile)
wrap both halves: `make build|test|clean` run the TS and Go sides, and
`make publish-go V=x.y.z` seds `V` into the `const Version` in
`go/hoover.go` (currently `0.1.7`), commits, tags `go/vX.Y.Z`, and (when
`gh` is present) creates a GitHub release. `make tags-go` lists the Go
tags. Local Go builds resolve the unpublished engine via the `replace`
in `go/go.mod` (a sibling checkout); there is no checked-in `go.work`.

## Tests

- [`ts/test/hoover.test.ts`](ts/test/hoover.test.ts) — behavior tests
  (delimiters, escapes, trim, consume, EOF, rule-context, fail-fast,
  custom token) against the mini grammar. `go/hoover_test.go` mirrors
  them.
- [`ts/test/doc-examples.test.ts`](ts/test/doc-examples.test.ts) — the
  shared tabnas doc-example harness: extracts fenced `js`/`javascript`
  blocks containing `// =>` assertions from this repo's READMEs and docs
  and runs them. Keep doc examples correct; mark illustrative blocks
  ` ```js ignore ` to opt them out.

## CI

[`.github/workflows/build.yml`](.github/workflows/build.yml) is a
**single TS-only `build` job** (Ubuntu/Windows/macOS, Node 24) — there is
no separate `build-go` job here. It:

- sets `git config --global core.autocrlf false` (LF line endings; CRLF
  corrupts fixtures across the tabnas repos),
- git-clones the tabnas closure (`parser debug json abnf railroad`) as
  siblings,
- runs `npm i && npm run build --if-present` in each of
  `parser debug json hoover abnf railroad` (topo order), then
- `npm test` in `hoover/ts`.

The Go suite is not run in CI — run it locally (`make test-go` / `cd go
&& go test ./...`).

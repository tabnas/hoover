# Agents Guide — @jsonic/hoover

## What this project is

hoover is a **syntax plugin for the [jsonic](https://github.com/jsonicjs/jsonic)
parser**. It adds configurable, block-delimited string parsing — what
the project calls *hoovering*: vacuuming up a run of source text
(including internal spaces and newlines that jsonic would otherwise
split on) between a start and end delimiter, with optional escape
handling and rule-context gating.

Two canonical examples drive the whole design and the shared fixtures:

- **triple-quoted strings** — `'''hello world'''` → `"hello world"`,
  preserving spaces and newlines.
- **end-of-line values** — an unquoted `key: some words here` up to a
  newline, `#`, or `;`, captured as one string.

Keep those two shapes in mind for every change; the `test/spec/*.tsv`
fixtures encode exactly this behavior in both runtimes.

## Repository map

| Path | What it is |
|---|---|
| [`ts/`](ts/) | **Canonical** TypeScript/JavaScript implementation — the `@jsonic/hoover` npm package. A single plugin in `ts/src/hoover.ts`. Requires `jsonic` >= 2 as a peer dependency. |
| [`go/`](go/) | Go port — module `github.com/jsonicjs/hoover/go`, a single `hoover.go`, depending on `github.com/jsonicjs/jsonic/go`. |
| [`test/spec/`](test/spec/) | Shared `.tsv` conformance fixtures (`input → expected-JSON`). Run by **both** the TypeScript suite and the Go suite. |

## Authority and alignment rules

1. **TypeScript is canonical.** When TS and Go disagree on parse
   behavior, TS wins; change Go to match. The lexer/parser engine
   underneath is jsonic itself, which is 1-based for row/column
   tracking in both languages — keep hoover consistent with that.
2. The shared fixtures in `test/spec/*.tsv` are the parity contract.
   Both suites load the same files and both must stay green. The Go
   suite resolves them at `../test/spec` (see `specDir()` in
   `go/hoover_tsv_test.go`); the TS suite resolves them at
   `../../test/spec` (see `loadTSV` in `ts/test/hoover-tsv.test.ts`).
3. Prefer adding a shared `input → expected` fixture over a one-off
   per-language assertion. When you add a fixture, wire it into **both**
   suites in the same change.
4. The configuration shape is the same in both languages: `block` is an
   **ordered array** of block definitions, each with a `name`. Blocks
   are tried in array order, so order is significant and must be
   preserved (the Go port must not iterate a map).
5. Once a block's start matches, the block is **committed**: failing to
   reach an end delimiter (or hitting a rejected escape) is an error,
   not a silent fall-through to the next block or matcher.

## Build / test

```bash
# TypeScript (from ts/)
npm install          # resolves the jsonic peer dependency
npm run build        # tsc --build src test  → dist/ and dist-test/
npm test             # node --test dist-test/*.test.js (includes the shared fixtures)

# Go (from go/)
go build ./... && go vet ./...
go test ./...        # includes the shared ../test/spec fixtures
```

`ts/Makefile` has combined `all`/`build`/`test` targets that drive both
languages. The TS suite runs against compiled output — always
`npm run build` after editing `ts/src/` or `ts/test/*.ts`.

## Documentation

Each implementation's docs are split by purpose; keep every file to one
job and do not mix them:

- **Learning** — the *Tutorials* section: one guided happy path per
  feature, built up step by step, no exhaustive option dumps.
- **Tasks** — the *How-to guides* section: short, self-contained
  recipes that solve one problem.
- **Explanation** — the *Explanation* section: how matching and matcher
  ordering actually work, and why.
- **Reference** — the *Reference* section: dry and complete type and
  function listings, no teaching.

`ts/doc/hoover-ts.md` and `go/doc/hoover-go.md` carry all four. Each
`README.md` is an **orientation hub** — what the package is, install,
one tiny example, and links out. Do not let a README grow into a
manual. Ground every factual claim and every code example against
`ts/src/hoover.ts`, `go/hoover.go`, and the fixtures before writing it;
the configuration API uses block **arrays** (`block: [{ name, ... }]`),
not a name-keyed map.

Working on the code itself? `ts/` and `go/` each have their own
`AGENTS.md` with layout and contribution notes.

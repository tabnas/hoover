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
fixtures encode exactly this behavior and are run by the TypeScript
suite. The Go suite checks the same behaviors against a small local
grammar, since it carries no grammar dependency.

## Repository map

| Path | What it is |
|---|---|
| [`ts/`](ts/) | **Canonical** TypeScript/JavaScript implementation — the `@jsonic/hoover` npm package. A single plugin in `ts/src/hoover.ts`. Requires `jsonic` >= 2 as a peer dependency. |
| [`go/`](go/) | Go port — module `github.com/jsonicjs/hoover/go`, a single `hoover.go`. Its **only** production dependency is the tabnas engine (`github.com/tabnas/parser/go`); tests bring their own tiny grammar. |
| [`test/spec/`](test/spec/) | Shared `.tsv` conformance fixtures (`input → expected-JSON`). Run by the TypeScript suite against the jsonic grammar. |

## Authority and alignment rules

1. **TypeScript is canonical.** When TS and Go disagree on parse
   behavior, TS wins; change Go to match. The lexer/parser engine
   underneath (tabnas) is 1-based for row/column tracking in both
   languages — keep hoover consistent with that.
2. The shared fixtures in `test/spec/*.tsv` are the behavior reference.
   The TS suite runs them against the jsonic grammar (resolved at
   `../../test/spec` via `loadTSV` in `ts/test/hoover-tsv.test.ts`). The
   Go side has no grammar dependency, so it cannot run the JSON
   fixtures; it verifies the same behaviors against a tiny local grammar
   (`go/minigrammar_test.go`). Equivalent configs must behave
   identically across the two.
3. The configuration shape is the same in both languages: `block` is an
   **ordered array** of block definitions, each with a `name`. Blocks
   are tried in array order, so order is significant and must be
   preserved (the Go port must not iterate a map).
4. Once a block's start matches, the block is **committed**: failing to
   reach an end delimiter (or hitting a rejected escape) is an error,
   not a silent fall-through to the next block or matcher.
5. Hoover is a **grammar-agnostic plugin**: it extends the host
   grammar's `val` rule, so it must be registered on an instance that
   already carries a grammar defining `val` (the engine itself ships
   none). Register the dependency grammar first, then the hoover plugin.
   Hoover **fails fast** with a clear error if the `val` rule is absent,
   rather than creating an empty one and failing confusingly later. Keep
   this guard in both runtimes. The Go port depends only on the engine;
   any grammar it tests against lives in test code.

## Build / test

```bash
# TypeScript (from ts/)
npm install          # resolves the jsonic peer dependency
npm run build        # tsc --build src test  → dist/ and dist-test/
npm test             # node --test dist-test/*.test.js (includes the shared fixtures)

# Go (from go/)
go build ./... && go vet ./...
go test ./...        # drives the plugin through go/minigrammar_test.go
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

# Agents Guide — @tabnas/hoover

## What this project is

hoover is a **syntax plugin for the [tabnas](https://github.com/tabnas/parser)
parser engine**. It adds configurable, block-delimited string parsing —
what the project calls *hoovering*: vacuuming up a run of source text
(including internal spaces and newlines the lexer would otherwise split
on) between a start and end delimiter, with optional escape handling and
rule-context gating.

hoover is **grammar-agnostic**: it extends the host grammar's `val`
rule, and its only dependency is the engine. Two canonical shapes drive
the design:

- **triple-quoted strings** — `'''hello world'''` → `"hello world"`,
  preserving spaces and newlines.
- **end-of-line / terminated values** — an unquoted run of text up to a
  newline, `#`, `;`, or end-of-input, captured as one string.

## Repository map

| Path | What it is |
|---|---|
| [`ts/`](ts/) | **Canonical** TypeScript/JavaScript implementation — the `@tabnas/hoover` npm package. A single plugin in `ts/src/hoover.ts`. Peer-depends only on the tabnas engine (`tabnas` >= 2). |
| [`go/`](go/) | Go port — module `github.com/tabnas/hoover/go`, a single `hoover.go`. Depends only on the tabnas engine (`github.com/tabnas/parser/go`). |

Each implementation's only production dependency is the engine; both
bring their own tiny test grammar (`val` + a parenthesised `group`).

## Authority and alignment rules

1. **TypeScript is canonical, and you work on it first.** Make every
   behavior change in `ts/` first, then port it to Go in the same
   change. When TS and Go disagree, TS wins; change Go to match. The
   engine (tabnas) is 1-based for row/column tracking in both languages
   — keep hoover consistent with that.
2. Neither runtime depends on a grammar package, so there are no shared
   JSON fixtures. Parity is kept by testing both ports against an
   **identical tiny local grammar** with **matching cases**:
   `ts/test/minigrammar.ts` and `go/minigrammar_test.go` define the same
   `val` + `group` grammar, and `ts/test/hoover.test.ts` /
   `go/hoover_test.go` assert the same inputs and outputs. Add a case to
   both in the same change.
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
   this guard in both runtimes. Both ports depend only on the engine;
   any grammar they test against lives in test code.

## Build / test

```bash
# TypeScript (from ts/)
npm install          # resolves the tabnas peer dependency
npm run build        # tsc --build src test  → dist/ and dist-test/
npm test             # node --test dist-test/*.test.js

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
`ts/src/hoover.ts`, `go/hoover.go`, and the tests before writing it;
the configuration API uses block **arrays** (`block: [{ name, ... }]`),
not a name-keyed map.

Working on the code itself? `ts/` and `go/` each have their own
`AGENTS.md` with layout and contribution notes.

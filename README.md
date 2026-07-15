# @tabnas/hoover

<!-- tabnas-badges -->
[![npm](https://tabnas.github.io/status/badges/hoover-npm.svg)](https://www.npmjs.com/package/@tabnas/hoover)
[![CI](https://github.com/tabnas/hoover/actions/workflows/ci.yml/badge.svg)](https://github.com/tabnas/hoover/actions/workflows/ci.yml)
[![go](https://tabnas.github.io/status/badges/hoover-go.svg)](https://pkg.go.dev/github.com/tabnas/hoover/go)
[![tabnas standard](https://tabnas.github.io/status/badges/hoover-standard.svg)](https://tabnas.github.io/status/)
<!-- /tabnas-badges -->

A syntax plugin for the [tabnas](https://github.com/tabnas/parser)
parser engine that adds string *hoovering* — block-delimited strings
with unquoted internal spaces. It is grammar-agnostic: it extends the
host grammar's `val` rule, and its only dependency is the engine.

```js
// '''...''' hoovers up everything between the delimiters, spaces and all:
//   j.parse("'''hello world'''")  ->  "hello world"
```

This repository contains two implementations:

| Path | Description |
|---|---|
| [`ts/`](ts/) | **Canonical** TypeScript / JavaScript implementation. |
| [`go/`](go/) | Go port (tracks the TS version). |

Each depends only on the tabnas engine and is tested against an identical
tiny local grammar (`val` + a parenthesised `group`); the two suites use
matching cases to stay aligned.

## Documentation

Four-quadrant [Diátaxis](https://diataxis.fr) docs in each language:

| | TypeScript | Go |
|---|---|---|
| Tutorial (learn) | [ts/doc/tutorial.md](ts/doc/tutorial.md) | [go/doc/tutorial.md](go/doc/tutorial.md) |
| How-to guide (tasks) | [ts/doc/guide.md](ts/doc/guide.md) | [go/doc/guide.md](go/doc/guide.md) |
| Reference (API) | [ts/doc/reference.md](ts/doc/reference.md) | [go/doc/reference.md](go/doc/reference.md) |
| Concepts (understand) | [ts/doc/concepts.md](ts/doc/concepts.md) | [go/doc/concepts.md](go/doc/concepts.md) |

See [`ts/README.md`](ts/README.md) and [`go/README.md`](go/README.md) for
language-specific orientation.

## License

MIT. Copyright (c) Richard Rodger.

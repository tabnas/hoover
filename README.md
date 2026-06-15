# @tabnas/hoover

A syntax plugin for the [tabnas](https://github.com/tabnas/parser)
parser engine that adds string *hoovering* — block-delimited strings
with unquoted internal spaces. It is grammar-agnostic: it extends the
host grammar's `val` rule, and its only dependency is the engine.

This repository contains:

| Path | Description |
|---|---|
| [`ts/`](ts/) | **Canonical** TypeScript / JavaScript implementation. |
| [`go/`](go/) | Go port. |

Each implementation depends only on the tabnas engine and is tested
against an identical tiny local grammar (`val` + a parenthesised
`group`); the two suites use matching cases to stay aligned.

See [`ts/README.md`](ts/README.md) for usage.

## License

MIT. Copyright (c) Richard Rodger.

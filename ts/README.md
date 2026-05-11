# @jsonic/hoover

A [Jsonic](https://github.com/jsonicjs/jsonic) syntax plugin that adds
configurable block-delimited string parsing. Define custom string
formats with start/end delimiters, escape sequences, and
context-sensitive matching.

Available for [TypeScript](doc/hoover-ts.md) and [Go](doc/hoover-go.md).

[![npm version](https://img.shields.io/npm/v/@jsonic/hoover.svg)](https://npmjs.com/package/@jsonic/hoover)
[![build](https://github.com/jsonicjs/hoover/actions/workflows/build.yml/badge.svg)](https://github.com/jsonicjs/hoover/actions/workflows/build.yml)
[![Coverage Status](https://coveralls.io/repos/github/jsonicjs/hoover/badge.svg?branch=main)](https://coveralls.io/github/jsonicjs/hoover?branch=main)
[![Known Vulnerabilities](https://snyk.io/test/github/jsonicjs/hoover/badge.svg)](https://snyk.io/test/github/jsonicjs/hoover)
[![DeepScan grade](https://deepscan.io/api/teams/5016/projects/22466/branches/663906/badge/grade.svg)](https://deepscan.io/dashboard#view=project&tid=5016&pid=22466&bid=663906)
[![Maintainability](https://api.codeclimate.com/v1/badges/10e9bede600896c77ce8/maintainability)](https://codeclimate.com/github/jsonicjs/hoover/maintainability)

| ![Voxgig](https://www.voxgig.com/res/img/vgt01r.png) | This open source module is sponsored and supported by [Voxgig](https://www.voxgig.com). |
| ---------------------------------------------------- | --------------------------------------------------------------------------------------- |


## Tutorials

Learn by building working examples from scratch.

- [Parse triple-quoted strings (TypeScript)](doc/hoover-ts.md#parse-triple-quoted-strings)
- [Parse triple-quoted strings (Go)](doc/hoover-go.md#parse-triple-quoted-strings)
- [Parse end-of-line values with comments (TypeScript)](doc/hoover-ts.md#parse-end-of-line-values-with-comments)
- [Parse end-of-line values with comments (Go)](doc/hoover-go.md#parse-end-of-line-values-with-comments)


## How-to guides

Solve specific problems with hoover configuration.

- [Control delimiter consumption (TypeScript)](doc/hoover-ts.md#control-delimiter-consumption) | [(Go)](doc/hoover-go.md#control-delimiter-consumption)
- [Add escape sequences (TypeScript)](doc/hoover-ts.md#add-escape-sequences) | [(Go)](doc/hoover-go.md#add-escape-sequences)
- [Restrict matching by rule context (TypeScript)](doc/hoover-ts.md#restrict-matching-by-rule-context) | [(Go)](doc/hoover-go.md#restrict-matching-by-rule-context)


## Explanation

Understand how hoover works under the hood.

- [How hoover matching works (TypeScript)](doc/hoover-ts.md#how-hoover-matching-works) | [(Go)](doc/hoover-go.md#how-hoover-matching-works)
- [Matcher ordering (TypeScript)](doc/hoover-ts.md#matcher-ordering) | [(Go)](doc/hoover-go.md#matcher-ordering)


## Reference

Complete API documentation for each language.

- [TypeScript API reference](doc/hoover-ts.md#reference)
- [Go API reference](doc/hoover-go.md#reference)


## Quick example

Parse triple-quoted strings that preserve internal whitespace and newlines:

**TypeScript**
```typescript
const j = Jsonic.make().use(Hoover, {
  block: {
    triplequote: {
      start: { fixed: "'''" },
      end: { fixed: "'''" },
    }
  }
})

j("{a: '''hello world'''}") // { a: 'hello world' }
```

**Go**
```go
j := jsonic.Make()
j.Use(hoover.Make(hoover.Options{
  Block: map[string]*hoover.Block{
    "triplequote": {
      Start: hoover.StartSpec{Fixed: []string{"'''"}},
      End:   hoover.EndSpec{Fixed: []string{"'''"}},
    },
  },
}))

j.Parse("{a: '''hello world'''}") // map[string]any{"a": "hello world"}
```


## License

MIT. Copyright (c) Richard Rodger and other contributors.

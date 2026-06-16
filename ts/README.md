# @tabnas/hoover

A [tabnas](https://github.com/tabnas/parser) parser-engine syntax plugin
that adds configurable block-delimited string parsing. Define custom
string formats with start/end delimiters, escape sequences, and
context-sensitive matching. Its only dependency is the engine; it
extends whatever grammar you register.

Available for [TypeScript](doc/hoover-ts.md) and [Go](doc/hoover-go.md).

[![npm version](https://img.shields.io/npm/v/@tabnas/hoover.svg)](https://npmjs.com/package/@tabnas/hoover)
[![build](https://github.com/tabnas/hoover/actions/workflows/build.yml/badge.svg)](https://github.com/tabnas/hoover/actions/workflows/build.yml)
[![Coverage Status](https://coveralls.io/repos/github/tabnas/hoover/badge.svg?branch=main)](https://coveralls.io/github/tabnas/hoover?branch=main)
[![Known Vulnerabilities](https://snyk.io/test/github/tabnas/hoover/badge.svg)](https://snyk.io/test/github/tabnas/hoover)
[![DeepScan grade](https://deepscan.io/api/teams/5016/projects/22466/branches/663906/badge/grade.svg)](https://deepscan.io/dashboard#view=project&tid=5016&pid=22466&bid=663906)
[![Maintainability](https://api.codeclimate.com/v1/badges/10e9bede600896c77ce8/maintainability)](https://codeclimate.com/github/tabnas/hoover/maintainability)

| ![Voxgig](https://www.voxgig.com/res/img/vgt01r.png) | This open source module is sponsored and supported by [Voxgig](https://www.voxgig.com). |
| ---------------------------------------------------- | --------------------------------------------------------------------------------------- |


## Tutorials

Learn by building working examples from scratch.

- [Parse triple-quoted strings (TypeScript)](doc/hoover-ts.md#parse-triple-quoted-strings)
- [Parse triple-quoted strings (Go)](doc/hoover-go.md#parse-triple-quoted-strings)
- [Parse a value up to a terminator (TypeScript)](doc/hoover-ts.md#parse-a-value-up-to-a-terminator)
- [Parse a value up to a terminator (Go)](doc/hoover-go.md#parse-a-value-up-to-a-terminator)


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

hoover extends a grammar you supply (`myGrammar` below must define the
`val` rule; the engine ships none).

**TypeScript**
```typescript
const j = new Tabnas()
  .use(myGrammar)
  .use(Hoover, {
    block: [
      { name: 'triplequote', start: { fixed: "'''" }, end: { fixed: "'''" } },
    ],
  })

j.parse("'''hello world'''") // "hello world"
```

**Go**
```go
j := tabnas.Make()
j.Use(myGrammar)
j.UseDefaults(hoover.Hoover, hoover.Defaults, map[string]any{
  "block": []*hoover.Block{
    {
      Name:  "triplequote",
      Start: hoover.StartSpec{Fixed: []string{"'''"}},
      End:   hoover.EndSpec{Fixed: []string{"'''"}},
    },
  },
})

j.Parse("'''hello world'''") // "hello world"
```


### Runnable example

A complete, self-contained example. hoover needs a host grammar that
defines the `val` rule, so this registers a tiny inline grammar (the
same shape as the test suite's `minigrammar.ts`: a single value, plus a
parenthesised `group`) before the hoover plugin, then parses
triple-quoted strings.

```js
const { Tabnas } = require('@tabnas/parser')
const { Hoover } = require('@tabnas/hoover')

// Tiny host grammar defining the `val` rule hoover plugs into.
function grammar(tn) {
  tn.options({
    fixed: { token: { '#OP': '(', '#CP': ')' } },
    rule: { start: 'val' },
  })
  tn.token('#OP')
  tn.token('#CP')

  // val: a scalar value, or a parenthesised group.
  tn.rule('val', (rs) => {
    rs.bo((r) => { r.node = undefined })
    rs.bc((r, ctx) => {
      r.node =
        undefined === r.node
          ? undefined === r.child.node
            ? 0 === r.os
              ? undefined
              : r.o0.resolveVal(r, ctx)
            : r.child.node
          : r.node
    })
    rs.open([{ s: '#OP', p: 'group', b: 1 }, { s: '#VAL' }])
    rs.close([{ s: '#ZZ' }, { s: '#CP', b: 1 }])
  })

  // group: '(' value ')' — yields the inner value.
  tn.rule('group', (rs) => {
    rs.bc((r) => { r.node = r.child.node })
    rs.open([{ s: '#OP', p: 'val' }])
    rs.close([{ s: '#CP' }])
  })
}

const j = new Tabnas()
  .use(grammar)
  .use(Hoover, {
    block: [
      { name: 'triplequote', start: { fixed: "'''" }, end: { fixed: "'''" } },
    ],
  })

j.parse("'''hello world'''")   // => "hello world"
j.parse("('''x''')")           // => "x"
```


## License

MIT. Copyright (c) Richard Rodger and other contributors.

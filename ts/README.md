# @tabnas/hoover

A [tabnas](https://github.com/tabnas/parser) parser-engine syntax plugin
that adds configurable block-delimited string parsing — *hoovering* up
unquoted strings with internal spaces. Define custom string formats with
start/end delimiters, escape sequences, and context-sensitive matching.
Its only dependency is the engine; it extends whatever grammar you
register.

```bash
npm install @tabnas/parser @tabnas/hoover
```

Requires `@tabnas/parser` `>=2` as a peer dependency. The engine ships no
grammar, and hoover is grammar-agnostic: it adds an alternate to the
`val` rule, so register a grammar that defines `val` **first**, then the
hoover plugin. If `val` is absent, `use(Hoover, …)` throws a clear error.

[![npm version](https://img.shields.io/npm/v/@tabnas/hoover.svg)](https://npmjs.com/package/@tabnas/hoover)
[![build](https://github.com/tabnas/hoover/actions/workflows/build.yml/badge.svg)](https://github.com/tabnas/hoover/actions/workflows/build.yml)

## Documentation

- [Tutorial](doc/tutorial.md) — zero to a working triple-quote parser.
- [How-to guide](doc/guide.md) — escapes, trimming, delimiter
  consumption, rule-context matching.
- [Reference](doc/reference.md) — every export, option, and type.
- [Concepts](doc/concepts.md) — how the matcher works and why.

The Go port lives in [`../go`](../go) with its own
[four-quadrant docs](../go/doc).

## Quick example

hoover extends a grammar you supply. This self-contained example
registers a tiny inline host grammar (a single value plus a parenthesised
`group`, the same shape as [`test/minigrammar.ts`](test/minigrammar.ts))
before the hoover plugin, then parses triple-quoted strings — which
preserve internal whitespace.

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

  tn.rule('val', (rs) => {
    rs.bo((r) => { r.node = undefined })
    rs.bc((r, ctx) => {
      r.node =
        undefined === r.node
          ? undefined === r.child.node
            ? 0 === r.os ? undefined : r.o0.resolveVal(r, ctx)
            : r.child.node
          : r.node
    })
    rs.open([{ s: '#OP', p: 'group', b: 1 }, { s: '#VAL' }])
    rs.close([{ s: '#ZZ' }, { s: '#CP', b: 1 }])
  })

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

j.parse("'''hello world'''")  // => "hello world"
j.parse("('''x''')")          // => "x"
```

See the [tutorial](doc/tutorial.md) for a step-by-step walkthrough.

## License

MIT. Copyright (c) Richard Rodger and other contributors.

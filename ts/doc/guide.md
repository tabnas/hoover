# How-to guide

Focused recipes for real hoover tasks. Each recipe shows the block
configuration that matters; the runnable blocks include the tiny host
grammar (a `val` rule plus a parenthesised `group`) that hoover needs to
plug into. In your own code, replace that grammar with whatever grammar
defines your `val` rule.

The host grammar used below is identical to the test suite's
[`test/minigrammar.ts`](../test/minigrammar.ts). To keep the recipes
readable, it is defined once per runnable block as `grammar(tn)`.

## Add escape sequences

Give a block an `escapeChar` and an `escape` map of escaped characters to
their replacements. The escape character lets a delimiter (or any
character) appear inside the value.

```js
const { Tabnas } = require('@tabnas/parser')
const { Hoover } = require('@tabnas/hoover')

function grammar(tn) {
  tn.options({ fixed: { token: { '#OP': '(', '#CP': ')' } }, rule: { start: 'val' } })
  tn.token('#OP'); tn.token('#CP')
  tn.rule('val', (rs) => {
    rs.bo((r) => { r.node = undefined })
    rs.bc((r, ctx) => {
      r.node = undefined === r.node
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
      {
        name: 'angle',
        start: { fixed: '<' },
        end: { fixed: '>' },
        escapeChar: '\\',
        escape: { n: '\n', '>': '>', '\\': '\\' },
      },
    ],
  })

j.parse('<a\\>b>')   // => "a>b"
j.parse('<a\\nb>')   // => "a\nb"
j.parse('<a\\\\b>')  // => "a\\b"
```

- `\>` becomes a literal `>` (so the end delimiter can appear in the value).
- `\n` is *mapped* to a real newline by the `escape` table.
- `\\` becomes a single backslash.

### Unknown escapes

By default `allowUnknownEscape` is `true`: an escape with no table entry
drops the escape character and keeps the next character literally.

```js
const { Tabnas } = require('@tabnas/parser')
const { Hoover } = require('@tabnas/hoover')

function grammar(tn) {
  tn.options({ fixed: { token: { '#OP': '(', '#CP': ')' } }, rule: { start: 'val' } })
  tn.token('#OP'); tn.token('#CP')
  tn.rule('val', (rs) => {
    rs.bo((r) => { r.node = undefined })
    rs.bc((r, ctx) => {
      r.node = undefined === r.node
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
      {
        name: 'angle',
        start: { fixed: '<' },
        end: { fixed: '>' },
        escapeChar: '\\',
        escape: { n: '\n', '>': '>', '\\': '\\' },
      },
    ],
  })

j.parse('<a\\zb>')  // => "azb"
```

To **reject** unknown escapes instead, set `allowUnknownEscape: false`;
the parse then throws an `invalid_escape` error. To **keep** the escape
character in the output for unknown escapes, set `preserveEscapeChar:
true` (so `\z` stays `\z`).

## Trim surrounding whitespace

By default hoover preserves every character between the delimiters,
including leading and trailing spaces. Set `trim: true` to strip
whitespace from the edges of the value while keeping internal spaces.

```js
const { Tabnas } = require('@tabnas/parser')
const { Hoover } = require('@tabnas/hoover')

function grammar(tn) {
  tn.options({ fixed: { token: { '#OP': '(', '#CP': ')' } }, rule: { start: 'val' } })
  tn.token('#OP'); tn.token('#CP')
  tn.rule('val', (rs) => {
    rs.bo((r) => { r.node = undefined })
    rs.bc((r, ctx) => {
      r.node = undefined === r.node
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
      { name: 'angle', start: { fixed: '<' }, end: { fixed: '>' }, trim: true },
    ],
  })

j.parse('<  hello  >')  // => "hello"
j.parse('< a b >')      // => "a b"
```

`< a b >` keeps the internal space between `a` and `b` but drops the edge
spaces.

## Control delimiter consumption

By default both the start and end delimiters are removed from the output.
Use `consume` on either `start` or `end` to change that:

- `consume: false` — leave the delimiter in the value (or, for an end
  delimiter, leave it in the source for another matcher / the host
  grammar to handle).
- `consume: [ ... ]` — consume only the listed delimiters; leave the
  others.

```js
const { Tabnas } = require('@tabnas/parser')
const { Hoover } = require('@tabnas/hoover')

function grammar(tn) {
  tn.options({ fixed: { token: { '#OP': '(', '#CP': ')' } }, rule: { start: 'val' } })
  tn.token('#OP'); tn.token('#CP')
  tn.rule('val', (rs) => {
    rs.bo((r) => { r.node = undefined })
    rs.bc((r, ctx) => {
      r.node = undefined === r.node
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

// Keep the start delimiter in the value.
const j1 = new Tabnas()
  .use(grammar)
  .use(Hoover, {
    block: [
      { name: 'a', start: { fixed: '<', consume: false }, end: { fixed: '>' } },
    ],
  })

j1.parse('<hi>')  // => "<hi"

// Consume only some of several possible start delimiters.
const j2 = new Tabnas()
  .use(grammar)
  .use(Hoover, {
    block: [
      {
        name: 'a',
        start: { fixed: ['<', '~'], consume: ['<'] },
        end: { fixed: '>' },
      },
    ],
  })

j2.parse('<hi>')  // => "hi"
j2.parse('~hi>')  // => "~hi"
```

The end side works the same way. Leaving the end delimiter unconsumed is
useful when the delimiter is a token the host grammar also needs to see —
for example, the closing `)` of a group:

```js
const { Tabnas } = require('@tabnas/parser')
const { Hoover } = require('@tabnas/hoover')

function grammar(tn) {
  tn.options({ fixed: { token: { '#OP': '(', '#CP': ')' } }, rule: { start: 'val' } })
  tn.token('#OP'); tn.token('#CP')
  tn.rule('val', (rs) => {
    rs.bo((r) => { r.node = undefined })
    rs.bc((r, ctx) => {
      r.node = undefined === r.node
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
      { name: 'a', start: { fixed: '~' }, end: { fixed: ')', consume: false } },
    ],
  })

j.parse('(~hi)')  // => "hi"
```

The block stops at `)`, leaving it for the group rule to close on.

## Stop at end-of-input or a terminator

To capture an unquoted value (spaces included) up to a terminator *or*
the end of input, list the empty string `''` among the end delimiters.
Combine with `consume` to choose which terminators are removed.

```js
const { Tabnas } = require('@tabnas/parser')
const { Hoover } = require('@tabnas/hoover')

function grammar(tn) {
  tn.options({ fixed: { token: { '#OP': '(', '#CP': ')' } }, rule: { start: 'val' } })
  tn.token('#OP'); tn.token('#CP')
  tn.rule('val', (rs) => {
    rs.bo((r) => { r.node = undefined })
    rs.bc((r, ctx) => {
      r.node = undefined === r.node
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
      { name: 'tilde', start: { fixed: '~' }, end: { fixed: [';', ''], consume: [';'] } },
    ],
  })

j.parse('~a b;')  // => "a b"
j.parse('~a b')   // => "a b"
```

### Newline-terminated values

The same end-delimiter mechanism handles line-based formats. List `'\n'`,
`'\r\n'` and `''` so a value ends at a Unix newline, a Windows newline,
or end-of-input:

```js
const { Tabnas } = require('@tabnas/parser')
const { Hoover } = require('@tabnas/hoover')

function grammar(tn) {
  tn.options({ fixed: { token: { '#OP': '(', '#CP': ')' } }, rule: { start: 'val' } })
  tn.token('#OP'); tn.token('#CP')
  tn.rule('val', (rs) => {
    rs.bo((r) => { r.node = undefined })
    rs.bc((r, ctx) => {
      r.node = undefined === r.node
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
      { name: 'line', start: { fixed: '~' }, end: { fixed: ['\n', '\r\n', ''] } },
    ],
  })

j.parse('~a b\n')    // => "a b"
j.parse('~a b\r\n')  // => "a b"
j.parse('~a b')      // => "a b"
```

## Restrict matching by rule context

Use `start.rule` to limit *where* a block matches, based on the rule the
parser is in when the lexer reaches it. This lets the same delimiter mean
different things in different positions. The rule names are whatever your
grammar defines — here, `group` (the parenthesised rule) and `val`.

```js
const { Tabnas } = require('@tabnas/parser')
const { Hoover } = require('@tabnas/hoover')

function grammar(tn) {
  tn.options({ fixed: { token: { '#OP': '(', '#CP': ')' } }, rule: { start: 'val' } })
  tn.token('#OP'); tn.token('#CP')
  tn.rule('val', (rs) => {
    rs.bo((r) => { r.node = undefined })
    rs.bc((r, ctx) => {
      r.node = undefined === r.node
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

// Match @...@ only when the current rule's parent is NOT `group`
// (i.e. at the top level, not inside parentheses).
const j = new Tabnas()
  .use(grammar)
  .use(Hoover, {
    block: [
      {
        name: 'at',
        start: { fixed: '@', rule: { parent: { exclude: ['group'] } } },
        end: { fixed: '@' },
      },
    ],
  })

j.parse('@hi@')    // => "hi"
j.parse('(@hi@)')  // => "@hi@"
```

At the top level the block matches and yields `hi`. Inside parentheses
the parent rule is `group`, which is excluded, so hoover does not match;
the host grammar's text matcher then reads `@hi@` literally.

You can also filter on the **current** rule (`current.include` /
`current.exclude`) and on the rule **state** (`state`: `'o'` for open —
the default — `'c'` for close, `'oc'` for either, or `''` to skip the
state check entirely).

## Use a custom token name

Each block produces a `#HV` token by default. Set `token` to register the
block under a different token name (useful when you want distinct tokens
per block, or to integrate with grammar code that references the token):

```js
const { Tabnas } = require('@tabnas/parser')
const { Hoover } = require('@tabnas/hoover')

function grammar(tn) {
  tn.options({ fixed: { token: { '#OP': '(', '#CP': ')' } }, rule: { start: 'val' } })
  tn.token('#OP'); tn.token('#CP')
  tn.rule('val', (rs) => {
    rs.bo((r) => { r.node = undefined })
    rs.bc((r, ctx) => {
      r.node = undefined === r.node
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
      { name: 'tq', token: '#XX', start: { fixed: "'''" }, end: { fixed: "'''" } },
    ],
  })

j.parse("'''x'''")  // => "x"
```

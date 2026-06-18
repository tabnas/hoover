# Tutorial: your first hoover string

This tutorial takes you from nothing to a working parser that understands
triple-quoted strings with spaces inside them. Follow it top to bottom;
every step builds on the last, and the final code runs as-is.

## What you will build

The tabnas engine, on its own, has no grammar and no string syntax. The
**hoover** plugin lets you "vacuum up" a run of characters — including
spaces and newlines — between a start and end delimiter, and turn it into
a single string value. By the end you will parse this:

```text
'''hello world'''
```

into the JavaScript string `"hello world"`, spaces and all.

## Step 1 — install the pieces

hoover is a plugin for the tabnas engine. You need both:

```bash
npm install @tabnas/parser @tabnas/hoover
```

`@tabnas/parser` is a peer dependency (version `>=2`). hoover's only
dependency is that engine.

## Step 2 — understand the one prerequisite

hoover does not define a grammar. It *extends* a grammar you supply, by
adding a new alternate to that grammar's `val` rule. So you must register
a grammar that defines `val` **before** you register hoover. If you
forget, `use(Hoover, ...)` throws a clear error instead of failing later.

For this tutorial we use a deliberately tiny grammar: a single value,
plus a parenthesised group. It is the same shape as the test suite's
`minigrammar.ts`. You do not need to understand its internals yet — copy
it as the host grammar that gives hoover a `val` rule to plug into.

## Step 3 — write the program

Create a file and paste this in. The `grammar` function is the host
grammar; the `Hoover` block defines the `'''...'''` syntax.

```js
const { Tabnas } = require('@tabnas/parser')
const { Hoover } = require('@tabnas/hoover')

// Tiny host grammar that defines the `val` rule hoover plugs into.
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

j.parse("'''hello world'''")  // => "hello world"
```

That is the whole happy path. `new Tabnas()` makes a bare engine, `.use(grammar)`
gives it a `val` rule, and `.use(Hoover, ...)` adds the triple-quote block.

## Step 4 — see what hoovering does for you

Run a few more inputs against the same `j` to see the point of the
plugin: the characters between the delimiters are taken verbatim.

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
      { name: 'triplequote', start: { fixed: "'''" }, end: { fixed: "'''" } },
    ],
  })

j.parse("'''hello world'''")  // => "hello world"
j.parse("'''  spaced  '''")   // => "  spaced  "
j.parse("('''x''')")          // => "x"
```

Notice three things:

1. The internal space in `hello world` is preserved — that is the
   "hoovering".
2. Leading and trailing spaces inside `'''  spaced  '''` are kept too:
   hoover does **not** trim by default.
3. `('''x''')` shows the block works wherever a value is expected — here,
   nested inside the host grammar's parentheses.

## Step 5 — terminate at end-of-input

Delimiters do not have to be symmetric. A common pattern is "start here,
run to the end of the input". List the empty string `''` among the end
delimiters to mean "or end-of-input":

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
      { name: 'tilde', start: { fixed: '~' }, end: { fixed: ['>', '!', ''] } },
    ],
  })

j.parse("~hello world")  // => "hello world"
j.parse("~a>")           // => "a"
j.parse("~a!")           // => "a"
```

`~hello world` has no closing delimiter, so the empty-string end marker
catches it at end-of-input. `~a>` and `~a!` stop at the first delimiter
they meet.

## You are done

You have:

- registered a host grammar and the hoover plugin in the right order;
- defined a block with start/end delimiters;
- hoovered up values containing spaces, including to end-of-input.

Where to go next:

- [Guide](guide.md) — focused recipes: escapes, trimming, controlling
  delimiter consumption, restricting where a block matches.
- [Reference](reference.md) — every option and type, precisely.
- [Concepts](concepts.md) — how the matcher works and why it is built
  this way.

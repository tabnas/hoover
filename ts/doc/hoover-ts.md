# Hoover plugin for the tabnas parser (TypeScript)

A syntax plugin that adds configurable block-delimited string parsing —
custom start/end delimiters, escape handling, and rule-context matching
— to the [tabnas](https://github.com/tabnas/parser) parser engine.

```bash
npm install @tabnas/hoover
```

Requires `@tabnas/parser` >= 2 as a peer dependency. hoover's only dependency is
the engine. The engine ships no grammar, and hoover is grammar-agnostic:
it adds an alternate to the `val` rule, so register a grammar that
defines `val` **first**, then the hoover plugin.

```ts
import { Tabnas } from '@tabnas/parser'
import { Hoover } from '@tabnas/hoover'

const j = new Tabnas()
  .use(myGrammar) // your grammar plugin; must define the `val` rule
  .use(Hoover, { block: [ /* ... */ ] })
```

`block` is an ordered array; blocks are tried in array order. If the
`val` rule is absent, `use(Hoover, …)` throws a clear error.

The examples below assume a grammar that parses a single value, plus a
parenthesised `group`, like the runnable
[`test/minigrammar.ts`](../test/minigrammar.ts) used by the test suite.


## Tutorials

### Parse triple-quoted strings

Add support for `'''...'''` strings that preserve internal whitespace
and newlines:

```typescript
const j = new Tabnas()
  .use(myGrammar)
  .use(Hoover, {
    block: [
      { name: 'triplequote', start: { fixed: "'''" }, end: { fixed: "'''" } },
    ],
  })

j.parse("'''hello world'''")  // "hello world"  (spaces preserved)
j.parse("'''line1\nline2'''") // "line1\nline2" (newlines preserved)
j.parse("('''x''')")          // "x"            (nested in a group)
```

### Parse a value up to a terminator

Capture an unquoted value (including spaces) up to a terminator or
end-of-input. Listing `''` among the end delimiters lets the value run
to EOF:

```typescript
const j = new Tabnas()
  .use(myGrammar)
  .use(Hoover, {
    block: [
      {
        name: 'line',
        start: { fixed: '~' },
        end: { fixed: [';', ''] },
        trim: true,
      },
    ],
  })

j.parse("~hello world;") // "hello world" (terminator)
j.parse("~ trimmed ")    // "trimmed"     (EOF, edges trimmed)
```


## How-to guides

### Control delimiter consumption

By default, both start and end delimiters are consumed (removed from
the output). Use `consume` to change this:

```typescript
// Don't consume the start delimiter (leave it for another matcher)
start: { fixed: '[', consume: false }

// Selectively consume only some end delimiters
end: {
  fixed: [';', '#', ''],
  consume: [';'],  // consume ';', leave '#' for another matcher
}
```

### Add escape sequences

Define an escape character and a mapping of escaped characters to
their replacements:

```typescript
block: [
  {
    name: 'myblock',
    start: { fixed: '<<<' },
    end: { fixed: '>>>' },
    escapeChar: '\\',
    escape: {
      'n': '\n',    // \n → newline
      't': '\t',    // \t → tab
      '\\': '\\',   // \\ → backslash
      '>': '>',     // \> → literal >
    },
    allowUnknownEscape: false,  // reject unrecognized \x sequences (default: true)
    preserveEscapeChar: false,  // strip the \ from output (default)
  },
]
```

`escapeChar` may be set without an `escape` map; unknown escapes then
follow `allowUnknownEscape` / `preserveEscapeChar`. A rejected escape
throws an `invalid_escape` parse error.

### Restrict matching by rule context

Use `start.rule` to limit when a block matches, based on the rule the
parser is in when the lexer reaches the block. The rule names are
whatever your grammar defines (e.g. `group`, or `pair`/`elem` in a
JSON-like grammar):

```typescript
start: {
  rule: {
    parent: { include: ['group'] },  // only when the current rule's parent is `group`
    // current: { include: ['val'] },
    // state: 'o',                    // 'o' = open (default), 'c' = close, '' = don't check
  }
}
```


## Explanation

### How hoover matching works

Hoover is a grammar-dependent plugin: it adds an alternate to the host
grammar's `val` rule, so a grammar providing that rule must be
registered first (the engine ships none). If the `val` rule is missing
(for example on a bare `new Tabnas()` instance), `use(Hoover, …)` throws
a clear error rather than silently creating an empty rule and failing
later.

Hoover registers a custom lexer matcher in the tokenization pipeline.
When the lexer encounters text, the matcher:

1. Iterates through configured blocks in array order.
2. For each block, checks the **rule context** (parent rule,
   current rule, rule state) against `start.rule` filters.
3. Checks for a **start delimiter** match if `start.fixed` is set.
4. If both checks pass, calls `parseToEnd` to scan forward until
   an **end delimiter** is found, handling escape sequences along
   the way.
5. Produces a hoover token (`#HV` by default) with the parsed value.

Once a start matches, the block is committed: if no end delimiter is
reached, or an escape is rejected, the matcher returns a bad token
rather than falling through to the next block.

A hoovered value that matches a keyword the host grammar defines
(via `value.def`, e.g. `true`/`false`/`null`) resolves to that value
rather than the string — but only when the host grammar enables value
lexing and defines those keywords. The bare engine defines none, so
plain text stays a string unless your grammar opts in.

### Matcher ordering

The `lex.order` option controls where hoover runs relative to the
engine's built-in matchers:

| Order | Matcher |
|-------|---------|
| 2e6 | Fixed tokens |
| 3e6 | Spaces |
| 4e6 | Lines |
| **4.5e6** | **Hoover (default)** |
| 5e6 | Strings |
| 6e6 | Comments |
| 7e6 | Numbers |
| 8e6 | Text |

Use a lower order to run before built-in matchers (e.g. triple-quote
before strings), or a higher order to run after (e.g. an end-of-line
value at `7.5e6`, after strings and numbers but before text).


## Reference

### `Hoover` (Plugin)

The plugin function. Register with `new Tabnas().use(Hoover, options)`,
after a grammar that defines the `val` rule.

### `HooverOptions`

```typescript
type HooverOptions = {
  block: Block[]            // ordered; blocks are tried in array order
  lex?: { order?: number }  // default: 4.5e6
  action?: AltAction
}
```

### `Block`

```typescript
type Block = {
  name: string
  start?: {
    fixed?: string | string[]
    consume?: null | boolean | string[]   // false = keep; array = consume only those
    rule?: {
      parent?: { include?: string[], exclude?: string[] }
      current?: { include?: string[], exclude?: string[] }
      state?: string  // '' | 'o' | 'c' | 'oc'; default 'o'
    }
  }
  end?: {
    fixed: string | string[]
    consume?: null | boolean | string[]
  }
  token?: string                  // default: '#HV'
  escapeChar?: string
  escape?: { [char: string]: string }
  allowUnknownEscape?: boolean    // default: true
  preserveEscapeChar?: boolean    // default: false
  trim?: boolean
}
```

### `StartResult`

```typescript
type StartResult = {
  match: boolean
  start?: string
}
```

### `ParseResult`

```typescript
type ParseResult = {
  done: boolean
  val: string
  bad?: Token
}
```

### `parseToEnd(lex, hvpnt, block, cfg)`

Exported function for parsing content from a position to a block end
delimiter. Used internally by the matcher but available for advanced
use.

### `Hoover.defaults`

```typescript
{
  block: [],
  lex: { order: 4.5e6 },
}
```

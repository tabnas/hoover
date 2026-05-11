# Hoover plugin for Jsonic (TypeScript)

A Jsonic syntax plugin that adds configurable block-delimited string
parsing with custom start/end delimiters, escape handling, and
rule-context matching.

```bash
npm install @jsonic/hoover
```

Requires `jsonic` >= 2 as a peer dependency.


## Tutorials

### Parse triple-quoted strings

Add support for `'''...'` strings that preserve whitespace and newlines:

```typescript
import { Jsonic } from 'jsonic'
import { Hoover } from '@jsonic/hoover'

const j = Jsonic.make().use(Hoover, {
  block: {
    triplequote: {
      start: { fixed: "'''" },
      end: { fixed: "'''" },
    }
  }
})

j("{a: '''hello world'''}")     // { a: 'hello world' }
j("'''line1\nline2'''")         // 'line1\nline2'
j("{a: '''\n  indented\n'''}") // { a: '\n  indented\n' }
```

### Parse end-of-line values with comments

Capture unquoted values (including spaces) up to end-of-line, with
`#` and `;` as comment/end markers:

```typescript
import { Jsonic } from 'jsonic'
import { Hoover } from '@jsonic/hoover'

const j = Jsonic.make().use(Hoover, {
  lex: { order: 7.5e6 },  // after string and number matchers
  block: {
    endofline: {
      start: {
        rule: {
          parent: { include: ['pair', 'elem'] },
        }
      },
      end: {
        fixed: ['\n', '\r\n', '#', ';', ''],
        consume: ['\n', '\r\n'],
      },
      escapeChar: '\\',
      escape: { '#': '#', ';': ';', '\\': '\\' },
      trim: true,
    }
  }
})

j("a: hello world\n")  // { a: 'hello world' }
j("x: value # comment") // { x: 'value' }
j("x: a\\#b")           // { x: 'a#b' }
```


## How-to guides

### Control delimiter consumption

By default, both start and end delimiters are consumed (removed from
the source). Use `consume` to change this:

```typescript
// Don't consume the end delimiter (leave it for another matcher)
end: {
  fixed: ['#', ';'],
  consume: false,
}

// Selectively consume only some end delimiters
end: {
  fixed: ['\n', '\r\n', '#', ';'],
  consume: ['\n', '\r\n'],  // consume newlines, leave # and ; alone
}
```

### Add escape sequences

Define an escape character and a mapping of escaped characters to
their replacements:

```typescript
block: {
  myblock: {
    start: { fixed: '<<<' },
    end: { fixed: '>>>' },
    escapeChar: '\\',
    escape: {
      'n': '\n',    // \n → newline
      't': '\t',    // \t → tab
      '\\': '\\',   // \\ → backslash
      '>': '>',     // \> → literal >
    },
    allowUnknownEscape: false,  // reject unrecognized \x sequences
    preserveEscapeChar: false,  // strip the \ from output
  }
}
```

### Restrict matching by rule context

Use `start.rule` to limit when a block matches based on the current
parser rule state:

```typescript
start: {
  rule: {
    parent: { include: ['pair', 'elem'] },  // only in object pairs or array elements
    // parent: { exclude: ['val'] },        // or exclude specific rules
    // current: { include: ['val'] },       // filter on current rule
    // state: 'o',                          // 'o' = open (default), 'c' = close, '' = don't check
  }
}
```


## Explanation

### How hoover matching works

Hoover registers a custom lexer matcher in Jsonic's tokenization
pipeline. When the lexer encounters text, the matcher:

1. Iterates through configured blocks in definition order.
2. For each block, checks the **rule context** (parent rule,
   current rule, rule state) against `start.rule` filters.
3. Checks for a **start delimiter** match if `start.fixed` is set.
4. If both checks pass, calls `parseToEnd` to scan forward until
   an **end delimiter** is found, handling escape sequences along
   the way.
5. Produces a hoover token (`#HV` by default) with the parsed value.

### Matcher ordering

The `lex.order` option controls where hoover runs relative to
Jsonic's built-in matchers:

| Order | Matcher |
|-------|---------|
| 2e6 | Fixed tokens (`{`, `}`, `[`, `]`, `:`, `,`) |
| 3e6 | Spaces |
| 4e6 | Lines |
| **4.5e6** | **Hoover (default)** |
| 5e6 | Strings |
| 6e6 | Comments |
| 7e6 | Numbers |
| 8e6 | Text |

Use a lower order to run before built-in matchers (e.g. triple-quote
before string), or a higher order to run after (e.g. end-of-line at
7.5e6, after strings and numbers but before text).


## Reference

### `Hoover` (Plugin)

The plugin function. Register with `Jsonic.make().use(Hoover, options)`.

### `HooverOptions`

```typescript
type HooverOptions = {
  block: { [name: string]: Block }
  lex?: { order?: number }  // default: 4.5e6
  action?: AltAction
}
```

### `Block`

```typescript
type Block = {
  start?: {
    fixed?: string | string[]
    consume?: null | boolean | string[]
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
  escapeChar?: string
  escape?: { [char: string]: string }
  allowUnknownEscape: boolean   // default: true
  preserveEscapeChar: boolean   // default: false
  trim: boolean
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
  block: {},
  lex: { order: 4.5e6 },
}
```

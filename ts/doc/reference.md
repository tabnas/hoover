# Reference

The complete public API of `@tabnas/hoover` (TypeScript / JavaScript).
This is the canonical implementation; the Go port tracks it.

```ts
import { Hoover, parseToEnd } from '@tabnas/hoover'
import type { Block, HooverOptions, ParseResult, StartResult } from '@tabnas/hoover'
```

## Exports

| Name | Kind | Description |
|---|---|---|
| `Hoover` | `Plugin` | The plugin function. Register with `.use(Hoover, options)`. |
| `Hoover.defaults` | `HooverOptions` | Default options merged when the plugin is registered. |
| `parseToEnd` | function | Low-level scan from a point to a block end delimiter. Used internally; exported for advanced use. |
| `Block` | type | A single block definition. |
| `HooverOptions` | type | The options object passed to the plugin. |
| `ParseResult` | type | Result returned by `parseToEnd`. |
| `StartResult` | type | Result of an internal start-delimiter match. |

## `Hoover` (Plugin)

```ts
const Hoover: Plugin
```

Registers hoover on a tabnas engine. **A grammar that defines the `val`
rule must be registered first.** On registration the plugin:

1. Throws `Error("@tabnas/hoover: the 'val' rule is missing; register a
   grammar that defines it before the hoover plugin")` if no usable `val`
   rule exists.
2. For each block, calls `tn.token(block.token)` to create the block's
   token, and adds a `val`-rule open alternate matching that token (once
   per distinct token name).
3. Registers a lexer matcher named `hoover` at `options.lex.order`.

Usage:

```ts
new Tabnas().use(myGrammar).use(Hoover, { block: [ /* ... */ ] })
```

## `HooverOptions`

```ts
type HooverOptions = {
  block: Block[]            // ordered; blocks are tried in array order
  lex?: { order?: number }  // matcher priority; default 4.5e6
  action?: AltAction        // optional action attached to the val-rule alternate
}
```

| Field | Type | Default | Notes |
|---|---|---|---|
| `block` | `Block[]` | `[]` | Block definitions. The matcher tries them in array order; the first whose start matches wins. |
| `lex.order` | `number` | `4.5e6` | Where the hoover matcher runs in the lexer pipeline (lower = earlier). See [Matcher ordering](concepts.md#matcher-ordering). |
| `action` | `AltAction` | — | An engine alt-action attached to each generated `val`-rule alternate. |

## `Block`

```ts
type Block = {
  name: string
  start?: {
    fixed?: string | string[]
    consume?: null | boolean | string[]
    rule?: {
      parent?: { include?: string[]; exclude?: string[] }
      current?: { include?: string[]; exclude?: string[] }
      state?: string
    }
  }
  end?: {
    fixed: string | string[]
    consume?: null | boolean | string[]
  }
  token?: string
  escapeChar?: string
  escape?: { [char: string]: string }
  allowUnknownEscape?: boolean
  preserveEscapeChar?: boolean
  trim?: boolean
}
```

### `name`

`string`. A label for the block. Stored on the produced token as
`token.use.block`. Required.

### `start`

How the block begins.

| Field | Type | Default | Behaviour |
|---|---|---|---|
| `start.fixed` | `string \| string[]` | — | The start delimiter(s). The first matching delimiter (in array order) opens the block. With no `fixed`, the start matches unconditionally (subject to `rule`). |
| `start.consume` | `null \| boolean \| string[]` | consume (`true`) | `false` keeps the matched delimiter in the value; an array consumes only the listed delimiters; `null`/absent consumes. |
| `start.rule` | object | — | Rule-context filters; see below. |

#### `start.rule`

Filters that decide whether the block may open, based on the rule the
parser is in when the lexer reaches the position (`lex.ctx.rule`). A
block opens only if **all** supplied filters pass.

| Field | Type | Default | Behaviour |
|---|---|---|---|
| `parent.include` | `string[]` | — | Pass if the current rule's *parent* rule name is in the list. |
| `parent.exclude` | `string[]` | — | Pass if the parent rule name is **not** in the list. |
| `current.include` | `string[]` | — | Pass if the *current* rule name is in the list. |
| `current.exclude` | `string[]` | — | Pass if the current rule name is **not** in the list. |
| `state` | `string` | `'o'` | Which rule states to match. `'o'` = open, `'c'` = close, `'oc'` = either; `''` skips the state check. The check passes if the current rule's state character is contained in this string. |

When no filters are supplied, the default behaviour is: state must be
open (`'o'`).

### `end`

How the block terminates.

| Field | Type | Default | Behaviour |
|---|---|---|---|
| `end.fixed` | `string \| string[]` | — | The end delimiter(s). The empty string `''` matches end-of-input. Multi-character delimiters are matched by first character then the remaining tail. |
| `end.consume` | `null \| boolean \| string[]` | consume (`true`) | `false` leaves the matched end delimiter in the source for later matchers/rules; an array consumes only the listed delimiters; `null`/absent consumes. |

### `token`

`string`, default `'#HV'`. The token name produced for hoovered values.
A `val`-rule alternate is added once per distinct token name across all
blocks.

### `escapeChar`

`string`. A single escape character. When present, an `escapeChar`
followed by another character is processed against the `escape` map (see
[escapes](#escape-handling)). May be set without an `escape` map, in
which case every escape is "unknown".

### `escape`

`{ [char: string]: string }`. Maps the character *after* the escape
character to its replacement. For example `{ n: '\n', '>': '>' }` maps
`\n` to a newline and `\>` to a literal `>`.

### `allowUnknownEscape`

`boolean`, default `true`. Controls escapes not present in the `escape`
map:

- `true` — the escape character is dropped and the following character is
  kept literally (e.g. `\z` → `z`), unless `preserveEscapeChar` keeps it.
- `false` — an unmapped escape raises an `invalid_escape` parse error.

### `preserveEscapeChar`

`boolean`, default `false`. When an unknown escape is allowed, `true`
keeps the escape character in the output (e.g. `\z` → `\z`) instead of
dropping it.

### `trim`

`boolean`, default `false`. When `true`, leading/trailing whitespace is
stripped from the value (internal whitespace is preserved). Also trims
the captured start text.

## Escape handling

While scanning the value, when the current character equals `escapeChar`:

1. If the next character is a key in `escape`, the mapped replacement is
   emitted and both characters are consumed.
2. Otherwise, if `allowUnknownEscape` is `true`, emit either the next
   character alone (default) or the escape char plus the next character
   (when `preserveEscapeChar` is `true`).
3. Otherwise return an `invalid_escape` bad token (the parse throws).

## Value resolution

After a value is captured, if the host grammar enables value lexing and
has a matching definition in `value.def` (for example `true`, `false`,
`null`), the string is replaced by that defined value. So a block that
hoovers `true` yields the boolean `true`, not the string `"true"` —
**only** when the host grammar defines those keywords. A bare engine
defines them via the grammar, not by hoover.

## Bad tokens / errors

| Condition | Result |
|---|---|
| `val` rule missing at registration | thrown `Error` |
| Start matched but no end delimiter reached | `invalid_text` bad token (parse throws) |
| Unmapped escape with `allowUnknownEscape: false` | `invalid_escape` bad token (parse throws) |

Once a block's start matches, the block is **committed**: a failure to
terminate does not fall through to the next block.

## `parseToEnd(lex, hvpnt, block, cfg)`

```ts
function parseToEnd(lex: Lex, hvpnt: Point, block: Block, cfg: Config): ParseResult
```

Scans `lex.src` from `hvpnt` to the block's end delimiter, applying
escapes, consumption, trimming, and value resolution. Advances `hvpnt`
past any consumed end delimiter. Exported for advanced use; the matcher
calls it internally.

## `ParseResult`

```ts
type ParseResult = {
  done: boolean   // true if an end delimiter (or EOF marker) was reached
  val: string     // the captured value (may be resolved to a non-string by the engine)
  bad?: Token     // present when a scan error (e.g. invalid escape) occurred
}
```

## `StartResult`

```ts
type StartResult = {
  match: boolean   // whether the block's start matched
  start?: string   // the captured start text (trimmed if trim is set)
}
```

## `Hoover.defaults`

```ts
Hoover.defaults = {
  block: [],
  lex: { order: 4.5e6 },  // before strings (5e6) and numbers (7e6)
}
```

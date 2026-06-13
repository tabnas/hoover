# Hoover plugin for the tabnas parser (Go)

A syntax plugin that adds configurable block-delimited string parsing —
custom start/end delimiters, escape handling, and rule-context matching
— to the [tabnas](https://github.com/tabnas/parser) parser engine.

```go
import (
  tabnas "github.com/tabnas/parser/go"
  hoover "github.com/jsonicjs/hoover/go"
)
```

```bash
go get github.com/jsonicjs/hoover/go
```

hoover's only dependency is the engine. The engine ships no grammar, and
hoover is grammar-agnostic: it adds an alternate to the `val` rule, so
register a grammar that defines `val` **first**, then the hoover plugin.
Register hoover with `UseDefaults`, which deep-merges `hoover.Defaults`
(the default lex order) under your options:

```go
j := tabnas.Make()
j.Use(myGrammar) // your grammar plugin; must define the `val` rule
j.UseDefaults(hoover.Hoover, hoover.Defaults, map[string]any{
  "block": []*hoover.Block{ /* ... */ },
})
```

`block` is an ordered `[]*hoover.Block`; blocks are tried in array
order. If the `val` rule is absent, `UseDefaults` returns a clear error.

The examples below assume a grammar that parses a single value, plus a
parenthesised `group`, like the runnable
[`minigrammar_test.go`](../minigrammar_test.go) used by the test suite.


## Tutorials

### Parse triple-quoted strings

Add support for `'''...'''` strings that preserve internal whitespace
and newlines:

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

j.Parse("'''hello world'''")  // "hello world"  (spaces preserved)
j.Parse("'''line1\nline2'''") // "line1\nline2" (newlines preserved)
j.Parse("('''x''')")          // "x"            (nested in a group)
```

### Parse a value up to a terminator

Capture an unquoted value (including spaces) up to a terminator or
end-of-input. Listing `""` among the end delimiters lets the value run
to EOF:

```go
j := tabnas.Make()
j.Use(myGrammar)
j.UseDefaults(hoover.Hoover, hoover.Defaults, map[string]any{
  "block": []*hoover.Block{
    {
      Name:  "line",
      Start: hoover.StartSpec{Fixed: []string{"~"}},
      End:   hoover.EndSpec{Fixed: []string{";", ""}},
      Trim:  true,
    },
  },
})

j.Parse("~hello world;") // "hello world" (terminator)
j.Parse("~ trimmed ")    // "trimmed"     (EOF, edges trimmed)
```


## How-to guides

### Control delimiter consumption

By default, both start and end delimiters are consumed (removed from
the output). `Consume` on `StartSpec` and `EndSpec` is `any`: `nil`
(the default) consumes; `false` keeps the delimiter for another matcher;
a `[]string` consumes only the listed delimiters.

```go
// Don't consume the start delimiter
Start: hoover.StartSpec{
  Fixed:   []string{"["},
  Consume: false,
}

// Selectively consume only some end delimiters
End: hoover.EndSpec{
  Fixed:   []string{";", "#", ""},
  Consume: []string{";"},  // consume ';', leave '#' for another matcher
}
```

### Add escape sequences

Define an escape character and a mapping of escaped characters to
their replacements:

```go
f := false
block := &hoover.Block{
  Name:       "myblock",
  Start:      hoover.StartSpec{Fixed: []string{"<<<"}},
  End:        hoover.EndSpec{Fixed: []string{">>>"}},
  EscapeChar: "\\",
  Escape: map[string]string{
    "n":  "\n",  // \n -> newline
    "t":  "\t",  // \t -> tab
    "\\": "\\",  // \\ -> backslash
    ">":  ">",   // \> -> literal >
  },
  AllowUnknownEscape: &f,  // reject unrecognized \x sequences (default: true)
  PreserveEscapeChar: false,
}
```

A rejected escape surfaces an `invalid_escape` parse error, matching
the TypeScript implementation.

### Restrict matching by rule context

Use `Start.Rule` to limit when a block matches, based on the rule the
parser is in when the lexer reaches the block. The rule names are
whatever your grammar defines (e.g. `group`, or `pair`/`elem` in a
JSON-like grammar):

```go
Start: hoover.StartSpec{
  Rule: &hoover.HooverRuleSpec{
    Parent: &hoover.HooverRuleFilter{
      Include: []string{"group"},  // only when the current rule's parent is `group`
    },
    // Current: &hoover.HooverRuleFilter{Include: []string{"val"}},
    // State: "o",  // "o" = open (default), "c" = close, "" = don't check
  },
}
```


## Explanation

### How hoover matching works

Hoover is a grammar-dependent plugin: it adds an alternate to the host
grammar's `val` rule, so a grammar providing that rule must be
registered first (the engine ships none). If the `val` rule is missing
(for example on a bare `tabnas.Make()` instance), `UseDefaults`/`Use`
returns a clear `error` rather than silently creating an empty rule and
failing later.

Hoover registers a custom lexer matcher in the tokenization pipeline
(via `SetOptions`, under `lex.match.hoover`) and prepends a `val`-rule
alternate for its token. When the lexer reaches a position, the matcher:

1. Iterates through the configured blocks in array order.
2. For each block, checks the **rule context** (`lex.Ctx.Rule`):
   parent rule, current rule, and rule state against the `Start.Rule`
   filters.
3. Checks for a **start delimiter** match if `Start.Fixed` is set.
4. If both checks pass, calls `parseToEnd` to scan forward until an
   **end delimiter** is found, handling escape sequences along the way.
5. Produces a hoover token (`#HV` by default) carrying the parsed value.

Once a start matches, the block is committed: if no end delimiter is
reached, or an escape is rejected, the matcher returns a bad token
rather than falling through to the next block.

### Matcher ordering

The `lex.order` option controls where hoover runs relative to the
engine's built-in matchers — the same scheme as the TypeScript plugin,
and the same `4.5e6` default:

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
value at `7.5e6`, after strings and numbers but before text). Use
`tabnas.Describe(j)` to confirm the registered matcher priority while
debugging.


## Reference

### `Hoover` (`tabnas.Plugin`)

The plugin value. Register with
`j.UseDefaults(hoover.Hoover, hoover.Defaults, opts)`.

### `Defaults`

```go
var Defaults = map[string]any{
  "lex": map[string]any{"order": 4500000}, // before string (5e6) and number (7e6)
}
```

Deep-merged under the caller's options by `UseDefaults`.

### Options map

```go
map[string]any{
  "block":  []*hoover.Block, // ordered; tried in array order
  "lex":    map[string]any{"order": int},
  "action": tabnas.AltAction, // optional val-rule alternate action
}
```

### `Block`

```go
type Block struct {
  Name               string
  Start              StartSpec
  End                EndSpec
  Token              string             // default "#HV"
  EscapeChar         string
  Escape             map[string]string
  AllowUnknownEscape *bool              // nil = default (true)
  PreserveEscapeChar bool               // default: false
  Trim               bool
}
```

### `StartSpec`

```go
type StartSpec struct {
  Fixed   []string        // start delimiter(s)
  Consume any             // nil = true; bool/*bool; []string = only those
  Rule    *HooverRuleSpec // rule context matching
}
```

### `EndSpec`

```go
type EndSpec struct {
  Fixed   []string // end delimiter(s)
  Consume any      // nil = true; bool; []string = only those
}
```

### `HooverRuleSpec`

```go
type HooverRuleSpec struct {
  Parent  *HooverRuleFilter
  Current *HooverRuleFilter
  State   string  // "" = don't check, "o"/"c"/"oc" = check; default "o"
}
```

### `HooverRuleFilter`

```go
type HooverRuleFilter struct {
  Include []string
  Exclude []string
}
```

### `Version`

The module version string (e.g. `"0.1.7"`).

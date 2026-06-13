# Hoover plugin for Jsonic (Go)

A Jsonic syntax plugin that adds configurable block-delimited string
parsing with custom start/end delimiters, escape handling, and
rule-context matching.

```go
import (
  "github.com/tabnas/parser/go/jsonic"
  hoover "github.com/jsonicjs/hoover/go"
)
```

hoover registers directly with the [tabnas](https://github.com/tabnas/parser)
parser engine. `jsonic.Make()` returns a `*tabnas.Tabnas` instance
carrying the relaxed-JSON grammar; hoover's types come from the engine
package `github.com/tabnas/parser/go`.

```bash
go get github.com/jsonicjs/hoover/go
```

The plugin is the `hoover.Hoover` value. Register it with
`jsonic.UseDefaults`, which deep-merges `hoover.Defaults` (the default
lex order) under your options:

```go
j := jsonic.Make()
j.UseDefaults(hoover.Hoover, hoover.Defaults, map[string]any{
  "block": []*hoover.Block{ /* ... */ },
})
```

`block` is an ordered `[]*hoover.Block`; blocks are tried in array
order.


## Tutorials

### Parse triple-quoted strings

Add support for `'''...'''` strings that preserve whitespace and
newlines:

```go
j := jsonic.Make()
j.UseDefaults(hoover.Hoover, hoover.Defaults, map[string]any{
  "block": []*hoover.Block{
    {
      Name:  "triplequote",
      Start: hoover.StartSpec{Fixed: []string{"'''"}},
      End:   hoover.EndSpec{Fixed: []string{"'''"}},
    },
  },
})

j.Parse("{a: '''hello world'''}")    // map[string]any{"a": "hello world"}
j.Parse("'''line1\nline2'''")        // "line1\nline2"
j.Parse("{a: '''\n  indented\n'''}") // map[string]any{"a": "\n  indented\n"}
```

### Parse end-of-line values with comments

Capture unquoted values (including spaces) up to end-of-line, with
`#` and `;` as comment/end markers. Run hoover after the string and
number matchers (`order: 7500000`) so quoted and numeric values are
still lexed normally:

```go
j := jsonic.Make()
j.UseDefaults(hoover.Hoover, hoover.Defaults, map[string]any{
  "lex": map[string]any{"order": 7500000},
  "block": []*hoover.Block{
    {
      Name: "endofline",
      Start: hoover.StartSpec{
        Rule: &hoover.HooverRuleSpec{
          Parent: &hoover.HooverRuleFilter{
            Include: []string{"pair", "elem"},
          },
        },
      },
      End: hoover.EndSpec{
        Fixed:   []string{"\n", "\r\n", "#", ";", ""},
        Consume: []string{"\n", "\r\n"},
      },
      EscapeChar: "\\",
      Escape:     map[string]string{"#": "#", ";": ";", "\\": "\\"},
      Trim:       true,
    },
  },
})

j.Parse("a: hello world\n")   // map[string]any{"a": "hello world"}
j.Parse("x: value # comment") // map[string]any{"x": "value"}
j.Parse("x: a\\#b")           // map[string]any{"x": "a#b"}
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
  Fixed:   []string{"\n", "\r\n", "#", ";"},
  Consume: []string{"\n", "\r\n"},  // consume newlines, leave # and ; alone
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

Use `Start.Rule` to limit when a block matches based on the current
parser rule state:

```go
Start: hoover.StartSpec{
  Rule: &hoover.HooverRuleSpec{
    Parent: &hoover.HooverRuleFilter{
      Include: []string{"pair", "elem"},  // only in object pairs or array elements
    },
    // Current: &hoover.HooverRuleFilter{Include: []string{"val"}},
    // State: "o",  // "o" = open (default), "c" = close, "" = don't check
  },
}
```


## Explanation

### How hoover matching works

Hoover registers a custom lexer matcher in Jsonic's tokenization
pipeline (via `SetOptions`, under `lex.match.hoover`) and prepends a
`val`-rule alternate for its token. When the lexer reaches a position,
the matcher:

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

The `lex.order` option controls where hoover runs relative to Jsonic's
built-in matchers — the same scheme as the TypeScript plugin, and the
same `4.5e6` default:

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
before strings), or a higher order to run after (e.g. end-of-line at
`7.5e6`, after strings and numbers but before text). Use
`jsonic.Describe(j)` to confirm the registered matcher priority while
debugging.


## Reference

### `Hoover` (`jsonic.Plugin`)

The plugin value. Register with
`j.UseDefaults(hoover.Hoover, hoover.Defaults, opts)`.

### `Defaults`

```go
var Defaults = map[string]any{
  "lex": map[string]any{"order": 4500000}, // before string (5e6) and number (7e6)
}
```

Deep-merged under the caller's options by `jsonic.UseDefaults`.

### Options map

```go
map[string]any{
  "block":  []*hoover.Block, // ordered; tried in array order
  "lex":    map[string]any{"order": int},
  "action": jsonic.AltAction, // optional val-rule alternate action
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

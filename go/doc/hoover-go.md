# Hoover plugin for Jsonic (Go)

A Jsonic syntax plugin that adds configurable block-delimited string
parsing with custom start/end delimiters, escape handling, and
rule-context matching.

```go
import (
  jsonic "github.com/jsonicjs/jsonic/go"
  hoover "github.com/jsonicjs/hoover/go"
)
```

```bash
go get github.com/jsonicjs/hoover/go
```


## Tutorials

### Parse triple-quoted strings

Add support for `'''...'''` strings that preserve whitespace and
newlines:

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

j.Parse("{a: '''hello world'''}")     // map[string]any{"a": "hello world"}
j.Parse("'''line1\nline2'''")         // "line1\nline2"
j.Parse("{a: '''\n  indented\n'''}") // map[string]any{"a": "\n  indented\n"}
```

### Parse end-of-line values with comments

Capture unquoted values (including spaces) up to end-of-line, with
`#` and `;` as comment/end markers:

```go
j := jsonic.Make()
j.Use(hoover.Make(hoover.Options{
  Block: map[string]*hoover.Block{
    "endofline": {
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
}))

j.Parse("a: hello world\n")   // map[string]any{"a": "hello world"}
j.Parse("x: value # comment") // map[string]any{"x": "value"}
j.Parse("x: a\\#b")           // map[string]any{"x": "a#b"}
```


## How-to guides

### Control delimiter consumption

By default, both start and end delimiters are consumed (removed from
the source). Use `Consume` to change this:

```go
// Don't consume the start delimiter
f := false
Start: hoover.StartSpec{
  Fixed:   []string{"["},
  Consume: &f,
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
t := true
f := false
Block: &hoover.Block{
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
pipeline via `MergeOptions`. When the lexer encounters text, the
matcher:

1. Iterates through configured blocks in definition order.
2. For blocks without a start delimiter, defers to built-in
   matchers for characters they would handle (strings, numbers,
   fixed tokens, spaces, comments).
3. Checks the **rule context** (`lex.Ctx.Rule`) against
   `Start.Rule` filters (parent rule, current rule, rule state).
4. Checks for a **start delimiter** match if `Start.Fixed` is set.
5. If both checks pass, calls `parseToEnd` to scan forward until
   an **end delimiter** is found, handling escape sequences along
   the way.
6. Produces a hoover token (`#HV` by default) with the parsed value.

### Matcher ordering

The Go Jsonic lexer runs custom matchers at priority < 2e6 (before
all built-in matchers). Hoover uses priority 1,500,000 and defers to
built-in matchers for characters they would handle when no start
delimiter is defined. This emulates the TypeScript behavior where
hoover can be ordered between built-in matchers.

| Priority | Matcher |
|----------|---------|
| **1.5e6** | **Hoover** |
| 2e6 | Fixed tokens (`{`, `}`, `[`, `]`, `:`, `,`) |
| 3e6 | Spaces |
| 4e6 | Lines |
| 5e6 | Strings |
| 6e6 | Comments |
| 7e6 | Numbers |
| 8e6 | Text |


## Reference

### `Make(opts Options) jsonic.Plugin`

Creates a Hoover plugin. Register with `j.Use(hoover.Make(opts))`.

### `Options`

```go
type Options struct {
  Block  map[string]*Block
  Action jsonic.AltAction
}
```

### `Block`

```go
type Block struct {
  Start              StartSpec
  End                EndSpec
  Token              string             // default "#HV"
  EscapeChar         string
  Escape             map[string]string
  AllowUnknownEscape *bool              // default: true
  PreserveEscapeChar bool               // default: false
  Trim               bool
}
```

### `StartSpec`

```go
type StartSpec struct {
  Fixed   []string          // start delimiter(s)
  Consume *bool             // nil = true
  Rule    *HooverRuleSpec   // rule context matching
}
```

### `EndSpec`

```go
type EndSpec struct {
  Fixed   []string // end delimiter(s)
  Consume any      // bool or []string; nil = true
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

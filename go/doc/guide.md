# How-to guide (Go)

Focused recipes for real hoover tasks in the Go port, `tabnashoover`.
Each recipe shows the block configuration that matters; assume a host
grammar that defines the `val` rule (a `val` rule plus a parenthesised
`group`, like the test suite's
[`minigrammar_test.go`](../minigrammar_test.go)) is already registered,
referred to as `grammar` below.

The Go API takes options as a `map[string]any`; blocks are
`[]*tabnashoover.Block`. Register with `UseDefaults` so
`tabnashoover.Defaults` (the lex order) is merged under your options.

## Add escape sequences

Give a block an `EscapeChar` and an `Escape` map of escaped characters to
their replacements. The escape character lets a delimiter (or any
character) appear inside the value.

```go
j := tabnas.Make()
j.Use(grammar)
j.UseDefaults(tabnashoover.Hoover, tabnashoover.Defaults, map[string]any{
    "block": []*tabnashoover.Block{
        {
            Name:       "angle",
            Start:      tabnashoover.StartSpec{Fixed: []string{"<"}},
            End:        tabnashoover.EndSpec{Fixed: []string{">"}},
            EscapeChar: "\\",
            Escape:     map[string]string{"n": "\n", ">": ">", "\\": "\\"},
        },
    },
})

j.Parse(`<a\>b>`)   // "a>b"   escaped end delimiter
j.Parse(`<a\nb>`)   // "a\nb"  mapped escape -> real newline
j.Parse(`<a\\b>`)   // "a\b"   escaped backslash
```

### Unknown escapes

By default `AllowUnknownEscape` is treated as `true` (the field is a
`*bool`; `nil` means default). An escape with no table entry drops the
escape character and keeps the next character:

```go
j.Parse(`<a\zb>`) // "azb"  unknown escape: backslash dropped
```

To **reject** unknown escapes, set `AllowUnknownEscape` to a pointer to
`false`; the parse then returns an `invalid_escape` error:

```go
f := false
block := &tabnashoover.Block{
    Name:               "angle",
    Start:              tabnashoover.StartSpec{Fixed: []string{"<"}},
    End:                tabnashoover.EndSpec{Fixed: []string{">"}},
    EscapeChar:         "\\",
    Escape:             map[string]string{">": ">"},
    AllowUnknownEscape: &f,
}
// j.Parse(`<a\zb>`) now returns a non-nil error.
```

To **keep** the escape character in the output for unknown escapes, set
`PreserveEscapeChar: true` (so `\z` stays `\z`).

## Trim surrounding whitespace

By default hoover preserves every character between the delimiters,
including leading/trailing spaces. Set `Trim: true` to strip whitespace
from the edges while keeping internal spaces.

```go
j.UseDefaults(tabnashoover.Hoover, tabnashoover.Defaults, map[string]any{
    "block": []*tabnashoover.Block{
        {
            Name:  "angle",
            Start: tabnashoover.StartSpec{Fixed: []string{"<"}},
            End:   tabnashoover.EndSpec{Fixed: []string{">"}},
            Trim:  true,
        },
    },
})

j.Parse(`<  hello  >`) // "hello"
j.Parse(`< a b >`)     // "a b"  internal space kept, edges trimmed
```

## Control delimiter consumption

By default both start and end delimiters are removed from the output.
`Consume` on `StartSpec` and `EndSpec` is `any`:

- `nil` (the default) — consume the delimiter.
- `false` — keep the delimiter (for a start, in the value; for an end,
  in the source for another matcher / the host grammar).
- a `[]string` — consume only the listed delimiters; leave the others.

```go
// Keep the start delimiter in the value.
j1 := tabnas.Make()
j1.Use(grammar)
j1.UseDefaults(tabnashoover.Hoover, tabnashoover.Defaults, map[string]any{
    "block": []*tabnashoover.Block{
        {
            Name:  "a",
            Start: tabnashoover.StartSpec{Fixed: []string{"<"}, Consume: false},
            End:   tabnashoover.EndSpec{Fixed: []string{">"}},
        },
    },
})
j1.Parse(`<hi>`) // "<hi"

// Consume only some of several possible start delimiters.
j2 := tabnas.Make()
j2.Use(grammar)
j2.UseDefaults(tabnashoover.Hoover, tabnashoover.Defaults, map[string]any{
    "block": []*tabnashoover.Block{
        {
            Name:  "a",
            Start: tabnashoover.StartSpec{Fixed: []string{"<", "~"}, Consume: []string{"<"}},
            End:   tabnashoover.EndSpec{Fixed: []string{">"}},
        },
    },
})
j2.Parse(`<hi>`) // "hi"   '<' consumed
j2.Parse(`~hi>`) // "~hi"  '~' kept
```

Leaving an end delimiter unconsumed is useful when it is a token the host
grammar also needs to see — for example the closing `)` of a group:

```go
j := tabnas.Make()
j.Use(grammar)
j.UseDefaults(tabnashoover.Hoover, tabnashoover.Defaults, map[string]any{
    "block": []*tabnashoover.Block{
        {
            Name:  "a",
            Start: tabnashoover.StartSpec{Fixed: []string{"~"}},
            End:   tabnashoover.EndSpec{Fixed: []string{")"}, Consume: false},
        },
    },
})
j.Parse(`(~hi)`) // "hi"  the ')' is left for the group rule to close on
```

## Stop at end-of-input or a terminator

To capture an unquoted value (spaces included) up to a terminator *or*
end-of-input, list the empty string `""` among the end delimiters.
Combine with `Consume` to pick which terminators are removed.

```go
j.UseDefaults(tabnashoover.Hoover, tabnashoover.Defaults, map[string]any{
    "block": []*tabnashoover.Block{
        {
            Name:  "tilde",
            Start: tabnashoover.StartSpec{Fixed: []string{"~"}},
            End:   tabnashoover.EndSpec{Fixed: []string{";", ""}, Consume: []string{";"}},
        },
    },
})

j.Parse(`~a b;`) // "a b"
j.Parse(`~a b`)  // "a b"
```

### Newline-terminated values

List `"\n"`, `"\r\n"` and `""` so a value ends at a Unix newline, a
Windows newline, or end-of-input:

```go
j.UseDefaults(tabnashoover.Hoover, tabnashoover.Defaults, map[string]any{
    "block": []*tabnashoover.Block{
        {
            Name:  "line",
            Start: tabnashoover.StartSpec{Fixed: []string{"~"}},
            End:   tabnashoover.EndSpec{Fixed: []string{"\n", "\r\n", ""}},
        },
    },
})

j.Parse("~a b\n")   // "a b"  newline consumed
j.Parse("~a b\r\n") // "a b"  CRLF consumed
j.Parse("~a b")     // "a b"  EOF
```

## Restrict matching by rule context

Use `Start.Rule` to limit *where* a block matches, based on the rule the
parser is in when the lexer reaches it. The rule names are whatever your
grammar defines — here `group` (the parenthesised rule) and `val`.

```go
// Match @...@ only when the current rule's parent is NOT `group`
// (i.e. at the top level, not inside parentheses).
j.UseDefaults(tabnashoover.Hoover, tabnashoover.Defaults, map[string]any{
    "block": []*tabnashoover.Block{
        {
            Name: "at",
            Start: tabnashoover.StartSpec{
                Fixed: []string{"@"},
                Rule: &tabnashoover.HooverRuleSpec{
                    Parent: &tabnashoover.HooverRuleFilter{Exclude: []string{"group"}},
                },
            },
            End: tabnashoover.EndSpec{Fixed: []string{"@"}},
        },
    },
})

j.Parse(`@hi@`)   // "hi"     top level: parent not group -> matches
j.Parse(`(@hi@)`) // "@hi@"   inside group: excluded -> text token
```

At the top level the block matches and yields `hi`. Inside parentheses
the parent rule is `group`, which is excluded, so hoover does not match;
the host grammar's text matcher reads `@hi@` literally.

You can also filter on the **current** rule (`Current.Include` /
`Current.Exclude`) and on the rule **state** (`State`: `"o"` for open —
the default — `"c"` for close, `"oc"` for either).

> Go note: unlike TS, `State: ""` does **not** mean "skip the state
> check" — the zero value cannot be distinguished from "unset", so it
> defaults to `"o"`. Set `State: "oc"` to also match in the close state.
> See [Concepts → Differences from the TS version](concepts.md#differences-from-the-ts-version).

## Use a custom token name

Each block produces a `#HV` token by default. Set `Token` to register the
block under a different token name:

```go
j.UseDefaults(tabnashoover.Hoover, tabnashoover.Defaults, map[string]any{
    "block": []*tabnashoover.Block{
        {
            Name:  "tq",
            Token: "#XX",
            Start: tabnashoover.StartSpec{Fixed: []string{"'''"}},
            End:   tabnashoover.EndSpec{Fixed: []string{"'''"}},
        },
    },
})
j.Parse(`'''x'''`) // "x"
```

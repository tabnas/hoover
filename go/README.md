# hoover (Go)

A Go port of [@tabnas/hoover](https://github.com/tabnas/hoover), a syntax
plugin for the [tabnas](https://github.com/tabnas/parser) parser engine
that adds configurable block-delimited string parsing — *hoovering* up
unquoted strings with internal spaces. Define custom string formats with
start/end delimiters, escape sequences, and context-sensitive matching.

This port tracks the canonical TypeScript implementation in
[`../ts`](../ts).

```bash
go get github.com/tabnas/hoover/go@latest
```

hoover's only dependency is the engine (`github.com/tabnas/parser/go`).
The engine ships no grammar, and hoover is grammar-agnostic: it adds an
alternate to the `val` rule, so register a grammar that defines `val`
**before** the hoover plugin. If that rule is absent, hoover returns a
clear `error`.

## Documentation

- [Tutorial](doc/tutorial.md) — zero to a working triple-quote parser.
- [How-to guide](doc/guide.md) — escapes, trimming, delimiter
  consumption, rule-context matching.
- [Reference](doc/reference.md) — every type, option, and the package API.
- [Concepts](doc/concepts.md) — how the matcher works, plus the
  **differences from the TS version**.

## Quick example

```go
package main

import (
    "fmt"

    tabnas "github.com/tabnas/parser/go"
    tabnashoover "github.com/tabnas/hoover/go"
)

func main() {
    j := tabnas.Make()
    j.Use(myGrammar) // your grammar plugin; must define the `val` rule
    j.UseDefaults(tabnashoover.Hoover, tabnashoover.Defaults, map[string]any{
        "block": []*tabnashoover.Block{
            {
                Name:  "triplequote",
                Start: tabnashoover.StartSpec{Fixed: []string{"'''"}},
                End:   tabnashoover.EndSpec{Fixed: []string{"'''"}},
            },
        },
    })

    out, err := j.Parse("'''hello world'''")
    if err != nil {
        panic(err)
    }
    fmt.Println(out) // hello world
}
```

For a minimal, runnable grammar to plug hoover into, see
[`minigrammar_test.go`](minigrammar_test.go) — the tiny `val` + `group`
grammar the test suite uses. The [tutorial](doc/tutorial.md) walks
through it step by step.

## License

MIT. Copyright (c) Richard Rodger and other contributors.

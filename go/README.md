# hoover (Go)

Version: 0.1.7

A Go port of [@tabnas/hoover](https://github.com/tabnas/hoover), a
syntax plugin for the [tabnas](https://github.com/tabnas/parser) parser
engine that adds configurable block-delimited string parsing. Define
custom string formats with start/end delimiters, escape sequences, and
context-sensitive matching.

hoover's only dependency is the engine itself
(`github.com/tabnas/parser/go`). The engine ships no grammar, and hoover
is grammar-agnostic: it adds an alternate to the `val` rule, so register
a grammar that defines `val` **before** the hoover plugin. hoover
returns a clear error if that rule is absent.

## Install

```bash
go get github.com/tabnas/hoover/go@latest
```

## Quick Example

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

    result, err := j.Parse("'''hello world'''")
    if err != nil {
        panic(err)
    }
    fmt.Println(result) // hello world
}
```

For a minimal, runnable grammar to plug hoover into, see
[`minigrammar_test.go`](minigrammar_test.go) — the tiny `val` + `group`
grammar the test suite uses.

## Documentation

- [Go API reference](../doc/hoover-go.md#reference)
- [Tutorials](../doc/hoover-go.md)

## License

MIT. Copyright (c) Richard Rodger and other contributors.

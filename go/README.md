# hoover (Go)

Version: 0.1.7

A Go port of [@jsonic/hoover](https://github.com/jsonicjs/hoover), a
[Jsonic](https://github.com/jsonicjs/jsonic) syntax plugin that adds
configurable block-delimited string parsing. Define custom string
formats with start/end delimiters, escape sequences, and
context-sensitive matching.

## Install

```bash
go get github.com/jsonicjs/hoover/go@latest
```

## Quick Example

```go
package main

import (
    "fmt"
    "github.com/tabnas/parser/go/jsonic"
    hoover "github.com/jsonicjs/hoover/go"
)

func main() {
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

    result, err := j.Parse("{a: '''hello world'''}")
    if err != nil {
        panic(err)
    }
    fmt.Println(result) // map[a:hello world]
}
```

## Documentation

- [Go API reference](../doc/hoover-go.md#reference)
- [Tutorials](../doc/hoover-go.md)

## License

MIT. Copyright (c) Richard Rodger and other contributors.

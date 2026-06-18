# Tutorial: your first hoover string (Go)

This tutorial takes you from nothing to a working parser that understands
triple-quoted strings with spaces inside them, using the Go port,
`tabnashoover`. Follow it top to bottom.

This is the Go port of the canonical TypeScript implementation. The
behaviour mirrors the [TS tutorial](../../ts/doc/tutorial.md); the
differences are in the Go API shape (see
[Concepts → Differences from the TS version](concepts.md#differences-from-the-ts-version)).

## What you will build

The tabnas engine, on its own, has no grammar and no string syntax. The
**hoover** plugin lets you "vacuum up" a run of characters — including
spaces and newlines — between a start and end delimiter, and turn it into
a single string value. By the end you will parse `'''hello world'''` into
the Go string `"hello world"`, spaces and all.

## Step 1 — install

hoover is a plugin for the tabnas engine. You need both modules:

```bash
go get github.com/tabnas/parser/go
go get github.com/tabnas/hoover/go@latest
```

Import them like this:

```go
import (
    tabnas "github.com/tabnas/parser/go"
    tabnashoover "github.com/tabnas/hoover/go"
)
```

hoover's only dependency is that engine.

## Step 2 — understand the one prerequisite

hoover does not define a grammar. It *extends* a grammar you supply, by
adding an alternate to that grammar's `val` rule. So you must register a
grammar that defines `val` **before** hoover. If you forget,
`UseDefaults`/`Use` returns a clear `error` instead of failing later.

For this tutorial we use a deliberately tiny grammar: a single value plus
a parenthesised group. It is the same shape as the test suite's
[`minigrammar_test.go`](../minigrammar_test.go). Copy it as the host
grammar that gives hoover a `val` rule to plug into.

## Step 3 — the host grammar

Define the tiny grammar as a `tabnas.Plugin`:

```go
func grammar(j *tabnas.Tabnas, _ map[string]any) error {
    op, cp := "(", ")"
    j.SetOptions(tabnas.Options{
        Fixed: &tabnas.FixedOptions{
            Token: map[string]*string{"#OP": &op, "#CP": &cp},
        },
        Rule: &tabnas.RuleOptions{Start: "val"},
        // Define a few keyword values so value resolution is deterministic
        // (the bare Go engine ships none).
        Value: &tabnas.ValueOptions{
            Def: map[string]*tabnas.ValueDef{
                "true":  {Val: true},
                "false": {Val: false},
                "null":  {Val: nil},
            },
        },
    })
    OP := j.Token("#OP")
    CP := j.Token("#CP")

    // val: a scalar value, or a parenthesised group.
    j.Rule("val", func(rs *tabnas.RuleSpec, _ *tabnas.Parser) {
        rs.AddBO(func(r *tabnas.Rule, ctx *tabnas.Context) {
            r.Node = tabnas.Undefined
        })
        rs.AddBC(func(r *tabnas.Rule, ctx *tabnas.Context) {
            if !tabnas.IsUndefined(r.Node) {
                return
            }
            if r.Child != nil && !tabnas.IsUndefined(r.Child.Node) {
                r.Node = r.Child.Node
                return
            }
            if r.OS == 0 {
                return
            }
            r.Node = r.O0.ResolveVal(r, ctx)
        })
        rs.AddOpen(
            &tabnas.AltSpec{S: [][]tabnas.Tin{{OP}}, P: "group", B: 1},
            &tabnas.AltSpec{S: [][]tabnas.Tin{tabnas.TinSetVAL}},
        )
        rs.AddClose(
            &tabnas.AltSpec{S: [][]tabnas.Tin{{tabnas.TinZZ}}},
            &tabnas.AltSpec{S: [][]tabnas.Tin{{CP}}, B: 1},
        )
    })

    // group: '(' value ')' — yields the inner value.
    j.Rule("group", func(rs *tabnas.RuleSpec, _ *tabnas.Parser) {
        rs.AddBC(func(r *tabnas.Rule, ctx *tabnas.Context) {
            if r.Child != nil {
                r.Node = r.Child.Node
            }
        })
        rs.AddOpen(&tabnas.AltSpec{S: [][]tabnas.Tin{{OP}}, P: "val"})
        rs.AddClose(&tabnas.AltSpec{S: [][]tabnas.Tin{{CP}}})
    })
    return nil
}
```

You do not need to understand its internals to use hoover — it just gives
hoover a `val` rule.

## Step 4 — register hoover and parse

Register the grammar, then hoover with a triple-quote block. Use
`UseDefaults`, which deep-merges `tabnashoover.Defaults` (the default lex
order) under your options:

```go
func main() {
    j := tabnas.Make()
    if err := j.Use(grammar); err != nil {
        panic(err)
    }
    if err := j.UseDefaults(tabnashoover.Hoover, tabnashoover.Defaults, map[string]any{
        "block": []*tabnashoover.Block{
            {
                Name:  "triplequote",
                Start: tabnashoover.StartSpec{Fixed: []string{"'''"}},
                End:   tabnashoover.EndSpec{Fixed: []string{"'''"}},
            },
        },
    }); err != nil {
        panic(err)
    }

    out, err := j.Parse("'''hello world'''")
    if err != nil {
        panic(err)
    }
    fmt.Println(out) // hello world
}
```

`tabnas.Make()` makes a bare engine, `Use(grammar)` gives it a `val`
rule, and `UseDefaults(Hoover, Defaults, ...)` adds the triple-quote
block. `Parse` returns `(any, error)`.

## Step 5 — see what hoovering does for you

Against the same `j`, these inputs show the point of the plugin — the
characters between the delimiters are taken verbatim:

```go
j.Parse("'''hello world'''") // "hello world"  (internal space preserved)
j.Parse("'''  spaced  '''")  // "  spaced  "   (no trim by default)
j.Parse("('''x''')")         // "x"            (nested in a group)
```

Note: hoover does **not** trim by default, and the block works wherever a
value is expected — here nested inside the grammar's parentheses.

## Step 6 — terminate at end-of-input

Delimiters need not be symmetric. List the empty string `""` among the
end delimiters to mean "or end-of-input":

```go
j.UseDefaults(tabnashoover.Hoover, tabnashoover.Defaults, map[string]any{
    "block": []*tabnashoover.Block{
        {
            Name:  "tilde",
            Start: tabnashoover.StartSpec{Fixed: []string{"~"}},
            End:   tabnashoover.EndSpec{Fixed: []string{">", "!", ""}},
        },
    },
})

j.Parse("~hello world") // "hello world"  (EOF terminates)
j.Parse("~a>")          // "a"            (first delimiter)
j.Parse("~a!")          // "a"            (second delimiter)
```

## You are done

You have registered a host grammar and hoover in the right order, defined
a block, and hoovered values containing spaces, including to
end-of-input.

Next:

- [Guide](guide.md) — escapes, trimming, delimiter consumption, rule
  context.
- [Reference](reference.md) — every type and option.
- [Concepts](concepts.md) — how it works, plus the differences from the
  TS version.

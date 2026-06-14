# Agents Guide — @jsonic/hoover (Go)

A Go port of `@jsonic/hoover`, the syntax plugin for block-delimited
string parsing. This is module `github.com/jsonicjs/hoover/go`, a single
`hoover.go`, and its **only production dependency is the
[tabnas](https://github.com/tabnas/parser) parser engine**
(`github.com/tabnas/parser/go`, imported as `tabnas`). The engine
supplies the plugin machinery types hoover uses (`Plugin`, `RuleSpec`,
`AltSpec`, `Tin`, `Lex`, `Token`, `Point`, `LexConfig`, `Options`, …).

hoover is **grammar-agnostic**: it adds an alternate to the host
grammar's `val` rule. The engine ships no grammar, so a grammar
providing `val` must be registered before hoover. hoover does not depend
on any grammar package — that keeps the dependency surface to the engine
alone.

## Authority

The TypeScript implementation (`../ts/src/hoover.ts`) is canonical for
parse behavior. When porting or fixing, read the TS source first and
mirror it. The TS suite is the parity reference; it runs the shared
`../test/spec/*.tsv` fixtures against the real jsonic grammar. The Go
side has no JSON grammar dependency, so it cannot run those JSON
fixtures — instead it verifies the same plugin behaviors (delimiters,
escapes, trim, consume, EOF, rule-context) against a tiny local grammar.
Keep the Go behavior identical to TS for equivalent configs.

## Layout

- `hoover.go` — the whole plugin:
  - `Hoover` — the `tabnas.Plugin` value; `Defaults` — the option
    defaults (lex order `4500000`); `Version`.
  - `Block`, `StartSpec`, `EndSpec`, `HooverRuleSpec`,
    `HooverRuleFilter` — the configuration types.
  - `matchStart` (rule + start-delimiter matching) and `parseToEnd`
    (forward scan, escapes, value resolution), mirroring the TS
    functions of the same names.
- `minigrammar_test.go` — a tiny bespoke grammar (`val` + parenthesised
  `group`, **not** JSON) and the `makeMini` helper. It exists only to
  give hoover something to plug into; production code never depends on
  it.
- `hoover_test.go` — behavior tests driving the plugin through the mini
  grammar, plus the fail-fast and custom-token cases.

## Registration API (differs from TS surface, same behavior)

Register a grammar first (provides `val`), then hoover with
`UseDefaults` so `Defaults` is deep-merged in:

```go
j := tabnas.Make()
j.Use(myGrammar) // must define the `val` rule
j.UseDefaults(hoover.Hoover, hoover.Defaults, map[string]any{
    "lex":   map[string]any{"order": 4500000}, // optional
    "block": []*hoover.Block{
        {Name: "triplequote",
            Start: hoover.StartSpec{Fixed: []string{"'''"}},
            End:   hoover.EndSpec{Fixed: []string{"'''"}}},
    },
})
```

`block` is an **ordered `[]*Block`** (not a map) — array order is the
match priority and must be preserved.

Type notes versus TS:
- `Consume` on both `StartSpec` and `EndSpec` is `any`: `nil` (=true),
  a `bool`/`*bool`, or a `[]string` to consume only the listed
  delimiters. This mirrors the TS `null | boolean | string[]`.
- `AllowUnknownEscape` is `*bool` (`nil` means the default, `true`).
- The `#HV` default token name and the `4.5e6` default lex order match
  TS exactly; use `tabnas.Describe(j)` to confirm matcher priority and
  token registration while debugging.

## Commands

```bash
go build ./... && go vet ./...
go test ./...
go test -run TestEscapes -v ./...
go test -coverpkg=./... -cover ./...
```

## Rules of the road

- Columns are 1-based and reset to `1` after a newline — the tabnas
  engine convention; keep it consistent with the engine and the TS port.
- Once a start matches, the block is committed: a parse failure surfaces
  a bad token (`lex.Bad(...)`), it does not fall through. `parseToEnd`
  signals this via `parseResult.err` (e.g. `invalid_escape`); the
  matcher turns a non-empty `err`, or an unterminated block, into a bad
  token, mirroring TS.
- Pointer option fields (`AllowUnknownEscape`, `StartSpec.Consume` when a
  `*bool`) follow the "`nil` = default" convention.
- Hoover depends on the host grammar's `val` rule. Register it on a
  grammar-bearing instance (`tabnas.Make().Use(grammar).UseDefaults(
  Hoover, …)`); the plugin inspects `j.RSM()` up front and returns a
  clear `error` if `val` is missing or has no alternates, instead of
  silently creating an empty rule.
- The local test grammar is deliberately minimal. When you need to test
  a new behavior, extend it just enough — do not pull in a full grammar
  package; the engine-only dependency is intentional.
- Two known Go/TS gaps, both rooted in the engine, not hoover:
  `HooverRuleSpec.State == ""` cannot mean "skip the state check" the way
  TS `state: ''` does (Go's zero value defaults to `"o"`); and the bare
  Go engine ships no `value.def` keywords (`true`/`false`/`null`) while
  the TS engine does, so value resolution needs the host grammar to
  define them (the test grammar does). Keep both documented if touched.

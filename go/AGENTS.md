# Agents Guide — @tabnas/hoover (Go)

A Go port of `@tabnas/hoover`, the syntax plugin for block-delimited
string parsing. This is module `github.com/tabnas/hoover/go`, a single
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
mirror it.

Both runtimes run the **same** shared `../test/spec/*.tsv` fixtures —
`parity_test.go` globs them here, `../ts/test/parity.test.ts` reads the
same directory there — and both drive them through an identical tiny
local grammar (`val` + a parenthesised `group`), not JSON. Those fixtures
are the parity contract: neither port can drift without one going red.
The in-language suites carry only the cases a fixture cannot express.
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
- `parity_test.go` — `TestSpec` globs `../test/spec/*.tsv` and runs every
  fixture through the mini grammar. The TS side runs the same files.
- `perf_test.go` — a relative check that reusing one configured instance
  beats rebuilding the plugin per parse.

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
- hoover never panics out of `Use`/`Parse`: the plugin recovers to a
  returned `error` and the matcher recovers to a bad token (Parse then
  returns an error). Keep new code panic-free anyway — guard every index
  and type assertion (use the comma-ok form); the recovers are a
  backstop, not a license to index unsafely.
- Hoover depends on the host grammar's `val` rule. Register it on a
  grammar-bearing instance (`tabnas.Make().Use(grammar).UseDefaults(
  Hoover, …)`); the plugin inspects `j.RSM()` up front and returns a
  clear `error` if `val` is missing or has no alternates, instead of
  silently creating an empty rule.
- The local test grammar is deliberately minimal. When you need to test
  a new behavior, extend it just enough — do not pull in a full grammar
  package; the engine-only dependency is intentional.
- Two Go/TS representation gaps, both rooted in the language or engine,
  not in hoover's behavior:
  - `HooverRuleSpec.State == ""` cannot mean "skip the state check" the
    way TS `state: ''` does, because Go's zero value is indistinguishable
    from unset (which defaults to `"o"`). The `StateAny` (`"*"`) sentinel
    is the Go spelling of it, and `ruleSpecFromAny` maps a data-shape
    `"state": ""` onto `StateAny`, so the shared fixtures exercise the
    same behavior in both runtimes. Only a Go **struct literal** needs
    `StateAny` written out. Note that a rulespec setting *only* the state
    skip — no parent/current filter — must still match: an absent
    condition is no constraint, not a failed one (hence the `matchRule
    *bool` tri-state and its `matchRule != nil && !*matchRule` guard).
  - The bare Go engine ships no `value.def` keywords
    (`true`/`false`/`null`) while the TS engine does, so value resolution
    needs the host grammar to define them (the test grammar does).

  Keep both documented if touched.

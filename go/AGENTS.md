# Agents Guide — @jsonic/hoover (Go)

A Go port of `@jsonic/hoover`, the [jsonic](https://github.com/jsonicjs/jsonic)
syntax plugin for block-delimited string parsing. This is module
`github.com/jsonicjs/hoover/go`, a single `hoover.go`, built on the Go
jsonic engine (`github.com/jsonicjs/jsonic/go`).

## Authority

The TypeScript implementation (`../ts/src/hoover.ts`) is canonical for
parse behavior. When porting or fixing, read the TS source first and
mirror it. The shared `../test/spec/*.tsv` fixtures are the contract:
the Go suite resolves them via `specDir()` in `hoover_tsv_test.go`
(`../test/spec`) and must keep every one green. A successful parse must
produce the same value as TypeScript.

## Layout

- `hoover.go` — the whole plugin:
  - `Hoover` — the `jsonic.Plugin` value; `Defaults` — the option
    defaults (lex order `4500000`); `Version`.
  - `Block`, `StartSpec`, `EndSpec`, `HooverRuleSpec`,
    `HooverRuleFilter` — the configuration types.
  - `matchStart` (rule + start-delimiter matching) and `parseToEnd`
    (forward scan, escapes, value resolution), mirroring the TS
    functions of the same names.
- `hoover_test.go` — unit tests mirroring `../ts/test/hoover.test.ts`.
- `hoover_tsv_test.go` — runs the shared fixtures plus the TSV loader.

## Registration API (differs from TS surface, same behavior)

Register with `UseDefaults` so `Defaults` is deep-merged in:

```go
j := jsonic.Make()
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
  TS exactly; use `jsonic.Describe(j)` to confirm matcher priority and
  token registration while debugging.

## Commands

```bash
go build ./... && go vet ./...
go test ./...                       # includes the shared ../test/spec fixtures
go test -run TestEndOfLine -v ./...
go test -coverpkg=./... -cover ./...
```

## Rules of the road

- Columns are 1-based and reset to `1` after a newline — the jsonic Go
  engine convention; keep it consistent with the engine and the TS port.
- Once a start matches, the block is committed: a parse failure surfaces
  a bad token (`lex.Bad(...)`), it does not fall through. `parseToEnd`
  signals this via `parseResult.err` (e.g. `invalid_escape`); the
  matcher turns a non-empty `err`, or an unterminated block, into a bad
  token, mirroring TS.
- Pointer option fields (`AllowUnknownEscape`, `StartSpec.Consume` when a
  `*bool`) follow the "`nil` = default" convention.
- Prefer adding a shared fixture under `../test/spec/` over a Go-only
  assertion when the case is `input → output`; wire it into the TS suite
  too.

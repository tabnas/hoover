# Reference (Go)

The complete public API of the Go port, package `tabnashoover`
(`github.com/tabnas/hoover/go`). This port tracks the canonical
TypeScript implementation; behaviour matches unless noted.

```go
import (
    tabnas "github.com/tabnas/parser/go"
    tabnashoover "github.com/tabnas/hoover/go"
)
```

## Exports

| Name | Kind | Description |
|---|---|---|
| `Hoover` | `tabnas.Plugin` | The plugin value. Register with `j.UseDefaults(Hoover, Defaults, opts)`. |
| `Defaults` | `map[string]any` | Default options deep-merged under the caller's options by `UseDefaults`. |
| `Version` | `string` (const) | The module version (e.g. `"0.1.7"`). |
| `Block` | struct | A single block definition. |
| `StartSpec` | struct | A block's start configuration. |
| `EndSpec` | struct | A block's end configuration. |
| `HooverRuleSpec` | struct | Rule-context conditions. |
| `HooverRuleFilter` | struct | Include/exclude lists. |

The matcher and scanning helpers (`matchStart`, `parseToEnd`, etc.) are
unexported; the matcher is installed for you when the plugin registers.

## `Hoover` (plugin)

```go
var Hoover tabnas.Plugin
```

Registers hoover on a tabnas engine. **A grammar that defines the `val`
rule must be registered first.** On registration the plugin:

1. Returns an `error` (`"hoover: the 'val' rule is missing; ..."`) if no
   usable `val` rule with open alternates exists.
2. For each block, calls `j.Token(block.Token)` to create the block's
   token, and prepends a `val`-rule open alternate matching that token
   (once per distinct token name).
3. Installs a lexer matcher named `hoover` at the configured lex order.

The plugin never panics: any unexpected panic in registration or in the
matcher is converted into a returned `error` / a bad token.

Register it via `UseDefaults` to merge `Defaults`:

```go
j.UseDefaults(tabnashoover.Hoover, tabnashoover.Defaults, map[string]any{
    "block": []*tabnashoover.Block{ /* ... */ },
})
```

A direct `j.Use(Hoover, opts)` also works; the lex order then falls back
to the built-in default (`4500000`) rather than `Defaults`.

## `Defaults`

```go
var Defaults = map[string]any{
    "lex": map[string]any{
        "order": 4500000, // before string (5e6) and number (7e6)
    },
}
```

Deep-merged under the caller's options by `UseDefaults`.

## Options map

The third argument to `UseDefaults` (or second to `Use`):

```go
map[string]any{
    "block":  []*tabnashoover.Block{ /* ... */ }, // ordered; tried in array order
    "lex":    map[string]any{"order": int},        // matcher priority
    "action": tabnas.AltAction,                    // optional val-rule alternate action
}
```

| Key | Type | Default | Notes |
|---|---|---|---|
| `block` | `[]*tabnashoover.Block` | — | Block definitions. The matcher tries them in slice order; the first whose start matches wins. |
| `lex` | `map[string]any{"order": int}` | `4500000` | Where the hoover matcher runs in the lexer pipeline (lower = earlier). |
| `action` | `tabnas.AltAction` | — | Engine alt-action attached to each generated `val`-rule alternate. |

A malformed `lex` (missing, wrong type, or a non-`int` order) falls back
to the default order instead of panicking.

## `Block`

```go
type Block struct {
    Name               string
    Start              StartSpec
    End                EndSpec
    Token              string             // default "#HV"
    EscapeChar         string
    Escape             map[string]string
    AllowUnknownEscape *bool              // nil = default (true)
    PreserveEscapeChar bool               // default false
    Trim               bool               // default false
}
```

| Field | Type | Default | Behaviour |
|---|---|---|---|
| `Name` | `string` | — | Label for the block. |
| `Start` | `StartSpec` | — | How the block begins. |
| `End` | `EndSpec` | — | How the block terminates. |
| `Token` | `string` | `"#HV"` | Token name produced for hoovered values. A `val` alternate is added once per distinct token name. |
| `EscapeChar` | `string` | — | A single escape character (first byte used). When set, escapes are processed. |
| `Escape` | `map[string]string` | — | Maps the character after `EscapeChar` to its replacement. |
| `AllowUnknownEscape` | `*bool` | `nil` → `true` | `nil` or `&true` allows unmapped escapes (escape char dropped); `&false` rejects them with an `invalid_escape` error. |
| `PreserveEscapeChar` | `bool` | `false` | When an unknown escape is allowed, `true` keeps the escape char in the output. |
| `Trim` | `bool` | `false` | Strip leading/trailing whitespace from the value (internal whitespace kept). |

The caller's `*Block` is **not** mutated: defaults and the internal token
id are applied to a copy.

## `StartSpec`

```go
type StartSpec struct {
    Fixed   []string        // start delimiter(s)
    Consume any             // nil = consume; bool/*bool; []string = only those
    Rule    *HooverRuleSpec // rule context matching
}
```

| Field | Type | Default | Behaviour |
|---|---|---|---|
| `Fixed` | `[]string` | — | Start delimiter(s); first match (in slice order) opens the block. With no `Fixed`, the start matches unconditionally (subject to `Rule`). |
| `Consume` | `any` | `nil` (consume) | `false` keeps the matched delimiter in the value; a `[]string` consumes only the listed delimiters. `bool` and `*bool` are accepted. |
| `Rule` | `*HooverRuleSpec` | `nil` | Rule-context filters. |

## `EndSpec`

```go
type EndSpec struct {
    Fixed   []string // end delimiter(s)
    Consume any      // nil = consume; bool; []string = only those
}
```

| Field | Type | Default | Behaviour |
|---|---|---|---|
| `Fixed` | `[]string` | — | End delimiter(s). The empty string `""` matches end-of-input. Multi-character delimiters are matched by first byte then the remaining tail. |
| `Consume` | `any` | `nil` (consume) | `false` leaves the matched end delimiter in the source; a `[]string` consumes only the listed delimiters. |

## `HooverRuleSpec`

```go
type HooverRuleSpec struct {
    Parent  *HooverRuleFilter
    Current *HooverRuleFilter
    State   string // "o"/"c"/"oc" = check; "" (unset) defaults to "o"
}
```

Filters that decide whether the block may open, based on the rule the
parser is in (`lex.Ctx.Rule`). A block opens only if **all** supplied
filters pass.

| Field | Type | Default | Behaviour |
|---|---|---|---|
| `Parent` | `*HooverRuleFilter` | `nil` | Match on the current rule's *parent* rule name. |
| `Current` | `*HooverRuleFilter` | `nil` | Match on the *current* rule name. |
| `State` | `string` | `""` → `"o"` | Which rule states to match. `"o"` open, `"c"` close, `"oc"` either. The check passes if the current rule's state character is contained in this string. |

> The zero value `State: ""` defaults to `"o"`; there is **no** Go
> equivalent of the TS "`state: ''` skips the check". To also match in
> the close state, use `"oc"`.

## `HooverRuleFilter`

```go
type HooverRuleFilter struct {
    Include []string
    Exclude []string
}
```

| Field | Behaviour |
|---|---|
| `Include` | Pass if the relevant rule name is in the list. |
| `Exclude` | Pass if the relevant rule name is **not** in the list. |

When both are set, both must pass.

## `Version`

```go
const Version = "0.1.7"
```

## Errors / bad tokens

| Condition | Result |
|---|---|
| `val` rule missing at registration | returned `error` from `Use`/`UseDefaults` |
| Start matched but no end delimiter reached | `invalid_text` bad token (`Parse` returns an error) |
| Unmapped escape with `AllowUnknownEscape: &false` | `invalid_escape` bad token (`Parse` returns an error) |
| Any unexpected panic | converted to an `error` (registration) or `invalid_text` bad token (matcher) |

Once a block's start matches, the block is **committed**: a failure to
terminate does not fall through to the next block.

## Value resolution

After a value is captured, if the host grammar enables value lexing
(`cfg.ValueLex`) and has a matching definition in `cfg.ValueDef` (for
example `true`, `false`, `null`), the string is replaced by that defined
value. The bare Go engine defines none, so define them on the instance
(via `Value.Def`) if you want resolution.

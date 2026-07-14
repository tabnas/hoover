# Concepts (Go)

How hoover works, how it relates to the tabnas engine, and the design
trade-offs — for the Go port, `tabnashoover`. For task recipes see the
[guide](guide.md); for the exact API see the [reference](reference.md).

This Go port tracks the canonical TypeScript implementation. The
conceptual model is identical; the API-level differences are collected in
[Differences from the TS version](#differences-from-the-ts-version) at
the end.

## What "hoovering" means

Most parsers treat an unquoted run of text as a single token only until
the first space — after that, the space is a separator. "Hoovering" is
the opposite intent: vacuum up a whole run of characters, **spaces and
newlines included**, between an explicit start and end delimiter, and
emit it as one string value.

This is what lets you write `'''hello world'''` and get the Go string
`"hello world"` rather than two tokens. The delimiters give the run a
definite beginning and end, so the spaces inside do not have to mean
"next token".

## hoover is a plugin, not a grammar

The tabnas engine on its own has no grammar: no JSON, no string syntax,
no value rules. hoover is a **syntax plugin**. It does two things on
registration:

1. **Adds a `val`-rule alternate.** The host grammar's `val` rule is
   where a single value is parsed. hoover prepends an alternate to that
   rule which accepts its block token (`#HV` by default). This is why a
   grammar defining `val` must be registered *first* — hoover has nothing
   to attach to otherwise, and `Use`/`UseDefaults` returns a clear
   `error` (it checks for a `val` rule with usable open alternates, not
   just the key) rather than silently producing a broken parser.

2. **Installs a lexer matcher.** hoover registers a matcher named
   `hoover` in the tokenization pipeline via `SetOptions` (under
   `lex.match.hoover`). This matcher recognises a block and produces the
   token.

The split matters: the **matcher** decides *what bytes become a token*,
and the **`val` alternate** lets the *parser accept that token* where a
value is expected.

## The matching algorithm

When the lexer reaches a position, the hoover matcher iterates the
configured blocks in slice order. For each block:

1. **Rule-context check** (`matchStart`). If `Start.Rule` is set, the
   matcher inspects `lex.Ctx.Rule`: its parent rule name, its own name,
   and its state (open/close). All supplied filters must pass. By default
   a block matches only when the rule is in the *open* state.

2. **Start-delimiter check** (`matchStart`). If `Start.Fixed` is set, the
   source at the current position must begin with one of the delimiters;
   the first match (in slice order) wins. Matched start text is consumed
   unless `Consume` says otherwise.

3. **Scan to end** (`parseToEnd`). If both checks pass, the matcher scans
   forward byte by byte, handling escapes, until it hits an end delimiter
   or the end-of-input marker (`""`). Multi-character delimiters are
   matched by first byte, then the remaining tail.

4. **Emit a token.** On success it produces the block's token carrying
   the captured value and advances the lexer point. On failure it returns
   a bad token.

If no block matches, the matcher returns `nil` and the lexer falls
through to the next matcher in the pipeline.

## Commitment: once a start matches, the block is bound

A key design choice: once a block's **start** matches, the matcher is
*committed* to that block. If the scan never finds an end delimiter, or
an escape is rejected, the matcher returns a **bad token**
(`invalid_text` or `invalid_escape`) — it does not quietly fall through
and try the next block. Failures are explicit rather than producing a
surprising alternative parse, and the engine does not backtrack across
blocks.

## The no-panic contract

The engine has a no-panic contract, and the Go port honours it
throughout: the plugin wraps registration in a `recover` that converts
any panic into a returned `error`, and the matcher wraps its body in a
`recover` that converts any panic into an `invalid_text` bad token. So a
malformed options shape or an unexpected runtime condition surfaces as a
normal `error` from `Use`/`UseDefaults` or `Parse`, never a crash.

## Matcher ordering

The lexer runs matchers in priority order, lowest number first. hoover's
lex order decides where it slots among the engine's built-in matchers:

| Order | Matcher |
|-------|---------|
| 2e6 | Fixed tokens |
| 3e6 | Spaces |
| 4e6 | Lines |
| **4.5e6** | **Hoover (default)** |
| 5e6 | Strings |
| 6e6 | Comments |
| 7e6 | Numbers |
| 8e6 | Text |

The default `4500000` places hoover **before** the string, number and
text matchers — deliberately, so a triple-quote block beats the ordinary
string matcher (which would otherwise claim the first quote), and an
end-of-line block beats the text matcher. Set `lex.order` higher (e.g.
`7500000`, after numbers but before text) for a block that should only
apply to what would otherwise be plain text. Use `tabnas.Describe(j)` to
confirm the registered matcher priority while debugging.

## Value resolution

After capturing the value, hoover consults the host grammar's value
definitions. If value lexing is enabled (`cfg.ValueLex`) and the captured
string matches a defined value (`cfg.ValueDef`, e.g. `true`/`false`/`null`
or whatever the grammar defines), the string is replaced by that value.
So hoovering `true` yields the Go `bool` `true`, and `null` yields `nil`.

This only happens when the **host grammar** opts in and defines those
keywords; hoover does not invent them.

## Why delimiters are fixed strings

Block delimiters are plain fixed strings (or slices of them), not regular
expressions. This keeps matching cheap (a substring check at the current
position) and predictable, and it composes cleanly with the slice-order
"first match wins" rule. Richer matching comes from listing several fixed
alternatives and from the rule-context filters, not per-character
pattern matching.

## Differences from the TS version

The Go port is behaviourally faithful to the canonical TypeScript
implementation, with these API-shape differences arising from Go's type
system and the engine's Go conventions:

- **Registration / options.** TS uses
  `tn.use(Hoover, { block, lex, action })` with a typed options object;
  Go uses `j.UseDefaults(Hoover, Defaults, map[string]any{ "block": ...,
  "lex": ..., "action": ... })` (or `j.Use(Hoover, opts)`), passing
  options as an untyped `map[string]any`. The default lex order lives in
  `tabnashoover.Defaults` and is deep-merged by `UseDefaults`; a direct
  `Use` falls back to the built-in default order (`4500000`).

- **Blocks are pointers.** TS `block` is `Block[]`; Go is
  `[]*tabnashoover.Block`.

- **`AllowUnknownEscape` is a `*bool`.** TS uses `allowUnknownEscape?:
  boolean` with default `true`. Go uses `*bool`: `nil` means default
  (`true`); set `&false` to reject unknown escapes. (TS can distinguish
  "absent" from `false` directly; Go needs the pointer.)

- **`Consume` is `any`.** Both TS (`null | boolean | string[]`) and Go
  (`any`, accepting `nil`/`bool`/`*bool`/`[]string`) express the same
  three behaviours; Go uses an interface value because it lacks a union
  type.

- **Rule `State: ""` does not skip the check.** In TS, `state: ''` means
  "do not check the rule state". In Go the zero value `""` is
  indistinguishable from "unset", so it **always defaults to `"o"`
  (open)**. To skip the state check (the TS `state: ''` behaviour), use
  the dedicated sentinel `StateAny` (`"*"`): `State: tabnashoover.StateAny`.

- **Errors instead of throws.** TS throws (`Error`, or a thrown parse
  error from a bad token). Go returns an `error` from `Use`/`UseDefaults`
  and from `Parse`, and never panics (see
  [The no-panic contract](#the-no-panic-contract)).

- **Value resolution surface.** Identical behaviour, but note the bare Go
  engine ships **no** default keyword values, whereas the TS engine ships
  `true`/`false`/`null`. In Go, define them on the instance (via
  `Value.Def`) for keyword resolution to apply.

- **A `Version` constant.** The Go package exposes `Version` (e.g.
  `"0.1.7"`); the TS package version lives in `package.json`.

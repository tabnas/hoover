# Concepts

How hoover works, how it relates to the tabnas engine, and the design
trade-offs behind it. For task recipes see the [guide](guide.md); for the
exact API see the [reference](reference.md).

## What "hoovering" means

Most parsers treat an unquoted run of text as a single token only until
the first space — after that, the space is a separator and the rest is a
new token. "Hoovering" is the opposite intent: vacuum up a whole run of
characters, **spaces and newlines included**, between an explicit start
and end delimiter, and emit it as one string value.

This is what lets you write:

```text
'''hello world'''
```

and get the string `"hello world"` rather than two tokens. The delimiters
are what give the run a definite beginning and end, so the spaces inside
do not have to mean "next token".

## hoover is a plugin, not a grammar

The tabnas engine on its own has no grammar: no JSON, no string syntax,
no value rules. Grammars and syntax extensions are supplied as plugins.
hoover is a **syntax plugin**. It does two things on registration:

1. **Adds a `val`-rule alternate.** The host grammar's `val` rule is the
   place where a single value is parsed. hoover adds an alternate to that
   rule which accepts its block token (`#HV` by default). This is why a
   grammar defining `val` must be registered *first* — hoover has nothing
   to attach to otherwise, and it throws a clear error rather than
   silently creating an empty rule that fails confusingly later.

2. **Registers a lexer matcher.** hoover installs a custom matcher named
   `hoover` in the tokenization pipeline (via `tn.options({ lex: { match:
   { hoover: ... } } })`). This matcher is what actually recognises a
   block and produces the token.

The split matters: the **matcher** decides *what bytes become a token*,
and the **`val` alternate** lets the *parser accept that token* where a
value is expected. Both halves are needed.

## The matching algorithm

When the lexer reaches a position, the hoover matcher iterates the
configured blocks in array order. For each block:

1. **Rule-context check** (`matchStart`). If `start.rule` is set, the
   matcher inspects the rule the parser is currently in (`lex.ctx.rule`):
   its parent rule name, its own name, and its state (open/close). All
   supplied filters must pass. By default a block matches only when the
   rule is in the *open* state.

2. **Start-delimiter check** (`matchStart`). If `start.fixed` is set, the
   source at the current position must begin with one of the delimiters.
   The first matching delimiter (in array order) wins. Matched start text
   is consumed unless `consume` says otherwise.

3. **Scan to end** (`parseToEnd`). If both checks pass, the matcher scans
   forward character by character, handling escapes, until it hits an end
   delimiter or the end-of-input marker (`''`). Multi-character
   delimiters are matched by first character, then the remaining tail.

4. **Emit a token.** On success it produces the block's token (carrying
   the captured value and `use.block = name`) and advances the lexer
   point. On failure it returns a bad token.

If no block matches, the matcher returns `undefined` and the lexer falls
through to the next matcher in the pipeline.

## Commitment: once a start matches, the block is bound

A key design choice: once a block's **start** matches, the matcher is
*committed* to that block. If the scan never finds an end delimiter, or
an escape is rejected, the matcher returns a **bad token** — it does not
quietly fall through and try the next block. This makes failures explicit
(`invalid_text` for an unterminated block, `invalid_escape` for a bad
escape) rather than producing a surprising alternative parse. It also
keeps the cost bounded: the engine does not backtrack across blocks.

## Matcher ordering

The lexer runs matchers in priority order, lowest number first. The
engine's built-in matchers sit at fixed priorities, and hoover's
`lex.order` decides where it slots in:

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

The default `4.5e6` places hoover **before** the string, number and text
matchers. That ordering is deliberate: a triple-quote block (`'''...'''`)
must be tried before the ordinary single/double-quote string matcher, or
the string matcher would claim the first quote. Likewise an end-of-line
"rest of the line" block must beat the text matcher.

You can move hoover by setting `lex.order`:

- **Lower** (e.g. before fixed tokens) — to pre-empt even the structural
  tokens. Rarely needed.
- **Higher** (e.g. `7.5e6`, after numbers but before text) — for a block
  that should only apply to what would otherwise be plain text, leaving
  genuine numbers and strings to the built-in matchers.

## Value resolution: when a hoovered string becomes a keyword

After capturing the value, hoover consults the host grammar's value
definitions. If value lexing is enabled and the captured string matches a
defined value (`true`, `false`, `null`, or whatever the grammar defines
in `value.def`), the string is replaced by that value. So hoovering
`true` yields the boolean `true`.

This only happens when the **host grammar** opts in and defines those
keywords; hoover does not invent them. It is the reason a hoover block
behaves consistently with the rest of the grammar's scalar values rather
than always forcing a string.

## Why delimiters are "fixed" strings

Block delimiters are plain fixed strings (or lists of them), not regular
expressions. This keeps matching cheap (a substring check at the current
position) and predictable, and it composes cleanly with the array-order
"first match wins" rule. Richer matching is expressed by listing several
fixed alternatives (multiple start or end delimiters) and by the
rule-context filters, rather than by per-character pattern matching.

## Relationship to the engine, in one picture

```text
your source text
      |
      v
  [ lexer pipeline ]   <- hoover matcher inserted at lex.order
      |                   recognises a block, emits a #HV token
      v
  [ parser / val rule ] <- hoover added a val alternate that accepts #HV
      |                   value resolution applies (true/false/null/...)
      v
  parsed value
```

hoover sits at both the lexing and parsing seams of the engine, which is
exactly what is required to introduce a brand-new token shape and have
the grammar accept it.

# tabnas-hoover (Rust)

Block-delimited string *hoovering* for the
[`tabnas`](https://github.com/tabnas/parser) parser engine, crate
`tabnas_hoover`.

Hoovering vacuums up a run of source text, internal spaces and newlines
included, between a start and an end delimiter, with optional escape
handling and rule-context gating. Two shapes drive the design: the
triple-quoted string (`'''hello world'''` gives `"hello world"`) and the
end-of-line value (an unquoted run of text up to a newline, `#`, `;` or
the end of the input).

This is the Rust port of the canonical TypeScript implementation in
[`../ts`](../ts); the TypeScript version is authoritative and this crate
tracks it. The Go port is in [`../go`](../go).

The plugin is grammar-agnostic. It installs one lexer matcher that emits
a `#HV` token and adds an alternate accepting that token to the host
grammar's `val` rule, so it is applied to a `Tabnas` instance that
already carries a grammar defining `val`, never to a bare engine. Its
only production dependency is the engine.

## Use

```rust,ignore
use tabnas::Tabnas;
use tabnas_hoover::{hoover, Block, HooverOptions};

let mut parser = Tabnas::new();
register_host_grammar(&mut parser); // any grammar that defines `val`

hoover(
    &mut parser,
    HooverOptions::new(vec![
        Block::delimited("triplequote", "'''", "'''"),
    ]),
)?;

let value = parser.parse("'''hello world'''")?; // "hello world"
```

A block is a [`Block`]: a name, a start (fixed delimiters, whether a
matched one is consumed, and an optional rule context), an end (fixed
delimiters, `""` for the end of the input, and consumption), an
optional escape character with an escape map, and a `trim` flag. Blocks
are tried in the order they are listed. Once a block's start matches it
is committed: a block that never reaches an end delimiter fails with
`invalid_text`, and a rejected escape fails with `invalid_escape`.

```rust
use tabnas_hoover::{Block, HooverRuleFilter, HooverRuleSpec};

fn main() {
    // An end-of-line value, only inside a `group` rule.
    let line = Block::new("line")
        .with_start(&["~"])
        .with_end(&["\n", "\r\n", ""])
        .with_rule(HooverRuleSpec {
            parent: Some(HooverRuleFilter::include(&["group"])),
            ..Default::default()
        });
    assert_eq!(line.name, "line");
}
```

The options the other runtimes take as plain data (`{"block": [...],
"lex": {"order": n}}`) are read by `HooverOptions::from_json` (behind the `serde_json` feature, off by default), with each
block's fields spelled as in TypeScript.

Every token a block emits carries `{block: <name>}` in its `use` bag,
and `HooverOptions::with_action` runs a callback on the `val` rule when
that token is matched.

## Install

The `tabnas` crate is not published to a registry, so the engine is
consumed as a **sibling checkout**, the standard tabnas development
model. Clone `https://github.com/tabnas/parser` next to this repository
and point at it:

```toml
[dependencies]
tabnas = { path = "../parser/rs" }
tabnas-hoover = { path = "../hoover/rs" }
```

Both entries are needed. A crate's dependencies are not passed on to its
dependents, so `tabnas-hoover` alone does not put `tabnas` in your
extern prelude.

## Differences from the canonical TypeScript

All deliberate, and each is a matter of typing rather than behaviour:

- **Options are a typed struct.** The alternate action is a Rust
  callback, and no serialized value can carry one, so `HooverOptions`
  travels in the plugin closure rather than in the engine's plugin
  option bag. The data shape is still accepted, through
  `HooverOptions::from_json`.
- **`consume` is an enum.** The TypeScript `null | boolean | string[]`
  is `Consume::All`, `Consume::Never` and `Consume::Only(list)`.
- **`state: ''` is `Some("")`.** A `None` state means the default,
  `"o"`; the empty string means "do not check the state", exactly as in
  TypeScript, so no sentinel is needed.
- **Registration failures are returned, not thrown.** A missing `val`
  rule, a host whose alt filter drops the block alternate, and a
  grammar the engine refuses all come back as `HooverError`, as they do
  in Go.
- **The block token is positioned at its start.** The canonical
  implementation records the end of the block as the token's position;
  this port, like Go, records where the block began. The value, the
  position of every later token and the engines' error messages are
  identical; the difference shows only in the token's own `sI`, `rI`
  and `cI`, as an action reads them.

One difference is behavioural, and it is recorded in
[`../DIVERGENCE.md`](../DIVERGENCE.md): after a *mapped* escape, the
canonical implementation and Go advance the row when the replacement
is a newline, while this port advances it when the escaped source
character is a newline. The hoovered value is the same; the row and
column of an error reported after such a block differ.

## Build and test

The engine is a path dependency on the sibling checkout, and so is the
`tabnas-support` fixture runner the tests use, so there is nothing to
fetch:

```bash
cargo test --all-targets
```

Or, from the repository root, `make test-rs`. For what CI would say,
including formatting and the lockfile check, run `ci/rust/run.sh`.

The suite runs the shared `../test/spec/*.tsv` conformance fixtures,
the same files the TypeScript and Go suites run, against the same tiny
host grammar (`val` plus a parenthesised `group`) as
`tests/common/mini_grammar.rs`. A row green in one runtime and red in
another is a failure, not a discrepancy.

## License

MIT.

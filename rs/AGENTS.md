# Agents Guide for rs/

The Rust port of the canonical TypeScript in [`../ts`](../ts). Read
[`../AGENTS.md`](../AGENTS.md) first: it holds the cross-runtime rules,
and this file only covers what is specific to this crate.

## Layout

| Path | |
|---|---|
| `src/lib.rs` | the whole port: option types, `plugin` / `hoover`, `match_start`, `parse_to_end`, `js_trim` |
| `tests/parity_test.rs` | the shared `../test/spec/*.tsv` fixtures through `tabnas_support::Runner` |
| `tests/hoover_test.rs` | the in-language behaviour tests, mirroring `../ts/test/hoover.test.ts` and `../go/hoover_test.go` |
| `tests/perf_test.rs` | instance reuse must beat rebuild-per-parse, mirroring `../go/perf_test.go` |
| `tests/version_test.rs` | `VERSION`, `Cargo.toml` and `ts/package.json` must agree |
| `tests/common/mini_grammar.rs` | the `val` + `group` host grammar, the Rust twin of `../ts/test/minigrammar.ts` and `../go/minigrammar_test.go` |
| `README.md` | the crate front page, prose-gated |

Crate `tabnas-hoover`, library `tabnas_hoover`. The engine crate
`tabnas` is a **path dependency on the sibling checkout**
(`../../parser/rs`), and so is the `tabnas-support` dev-dependency
(`../../support/rs`). Neither is published, so there is no registry
version to fall back on. Clone both next to this repo.

```bash
CARGO_TERM_COLOR=never cargo build --all-targets
cargo test --all-targets
cargo test --doc
cargo clippy --all-targets --all-features -- -D warnings
cargo fmt
```

`make test-rs` from the repo root is the fast loop; `ci/rust/run.sh` is
the full gate (fmt check, build, tests, doctests, clippy, the lockfile
check and the MSRV pin).

## How it is wired

Hoover is a LEXER MATCHER plugin, and it is registered through the
engine's serialized document, not through typed option fields:

- `install` registers a factory under a unique reference name with
  `lex_match_factory_ref`, then installs one `GrammarSpec` holding
  `options.lex.match.hoover = {order, make: "@hoover-matcher-N"}` and
  `rule.val.open = [alts]`. A bare `open` array PREPENDS, which is what
  `rs.open(...)` does in TypeScript and `PrependOpen` in Go.
- The factory sees the resolved `Options` and snapshots the
  `value.lex` flag and the `value.definitions` table into a
  `MatcherConfig`. That is the `cfg` argument the canonical
  `parseToEnd` reads; the Rust matcher cannot reach the options at
  lex time, so it is captured at setup.
- The matcher is an `ImperativeLexMatcher`: it gets `&mut Lexer`,
  `&mut Rule` and `&mut Context`. `match_start` reads the rule's
  `name`, `state` and `parent_rule` for the rule-context gate;
  `parse_to_end` scans `lexer.remaining()` by byte offset, and the
  matcher then advances the cursor with `advance_chars`, so the
  ENGINE tracks row and column. TypeScript and Go carry their own
  `rI`/`cI` bookkeeping through the scan; here the lexer's counters
  are authoritative, which is what keeps the `ERROR:<row>:<col>`
  rows in `escapes.tsv` green. It is also the one place this port
  cannot follow TypeScript: after a MAPPED escape TS and Go bump the
  row when the replacement is a newline, whatever the source was, and
  the `Lexer` exposes no way to move its row counter. See
  `../DIVERGENCE.md` and `row_after_a_mapped_escape_follows_the_source`
  in `tests/hoover_test.rs`.
- `match_start` sees `parent_rule: None` on the start rule. TypeScript
  gives that rule a sentinel parent whose name is the empty string, so
  a `parent.include` list matches it only with a `""` entry and a
  `parent.exclude` list rejects it only with one; `match_start` reads
  the missing parent as `""` for exactly that reason. Do not substitute
  any other name.
- The token is built with `lexer.token(...)` from the point captured
  BEFORE advancing, and `use_data_mut()` carries `{block: name}`, the
  `tkn.use = { block }` the TypeScript sets.
- A rejected escape is `lexer.bad_span("invalid_escape", at, at + 1)`;
  an unterminated block is `lexer.bad_span("invalid_text", start,
  start + width_of_start_delimiter)`, the span TypeScript passes to
  `lex.bad`. Scalar positions, not bytes: `bad_span` indexes by
  Unicode scalar.

Reference names carry a per-install counter (`@hoover-matcher-3`,
`@hoover-action-3`). Two hoover registrations on one instance, or the
plugin re-running on a derived instance, must not overwrite each
other's action. The matcher NAME stays `hoover`, so a second
registration replaces the first's matcher, exactly as
`lex.match.hoover` does in TypeScript.

## The include-filter check is reproduced, not observed

TypeScript reads the surviving alts back off `tn.rule().val.def.open`
after its `tn.options()` call re-runs the engine's alt filter. This
engine applies `rule.include` / `rule.exclude` when a parse is
prepared, not when a grammar is installed, so there is nothing to read
back. `install` therefore applies the same predicate the engine does
(`groups_enabled`, a copy of `parser.rs`'s) to the `val` alts it can
see, and refuses when the hoover alt would not survive. The unit test
`groups_enabled_mirrors_the_engine_filter` pins the copy; if the
engine's filter changes shape, change both.

The alt carries the host's `rule.include` tags (joined with commas, the
engine's group syntax), so in practice the refusal fires only when the
host also EXCLUDES one of its own include tags.
`hoover_refuses_a_host_whose_filter_excludes_the_block_alternate`
builds exactly that host.

## Where TypeScript relies on `undefined`

`parse_to_end` builds `endchars` from the end delimiters as
`Vec<Option<char>>`: the `""` (end-of-input) delimiter has no first
character, and `None` is what nothing in the source equals. The
escape character is `Option<char>` for the same reason. Do not spell
either as `'\0'`: a literal NUL in the source then reads as
end-of-input, which `end-delimiters.tsv` and `escapes.tsv` both pin.

## `trim` is JavaScript's, not `char::is_whitespace`

`js_trim` enumerates the ECMA-262 WhiteSpace + LineTerminator set. It
differs from Unicode White_Space in both directions (U+0085 NEL is
White_Space and not trimmed; U+FEFF is trimmed and not White_Space),
and `trim.tsv` pins both. Do not simplify it to `str::trim`.

## The fixture runner

`tests/parity_test.rs` bypasses the runner's input decoding and
decodes the RAW `input` cell itself, because hoover's fixtures need a
sixth escape, `\uXXXX`, which the shared codec passes through on
purpose. `spec_unescape` is kept in step with `unescapeHoover`
(TypeScript) and `specUnescape` (Go). The `opts` column is read with
`HooverOptions::from_json` (behind the `serde_json` feature, off by default) and a fresh parser is built per row, as the
other runners do.

`ERROR:<row>:<col>` cells name a POSITION and are matched against the
rendered error message with `Runner::match_error`, since the support
crate's own position channel is spelled `@<row>:<col>`. Two probe
tests pin that the match bites in both directions.

## Prose

`README.md` is a published page: follow
[`../docs/STYLE-GUIDE.md`](../docs/STYLE-GUIDE.md), no em dashes in
prose, no first person singular, and no link from it to any
`AGENTS.md`. This file is internal and may be blunt.

## The README is doctested

`src/lib.rs` includes `README.md` as crate documentation under
`#[cfg(doctest)]`, so `cargo test --doc` compiles and runs every `rust`
fence in the README exactly as it appears on the page. Keep each fence a
complete `fn main` example (no top-level `?`, no hidden `# ` lines), and
expect `readme_examples (line N)` entries in the doctest output, one per
fence.

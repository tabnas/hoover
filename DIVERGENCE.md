# Divergences

TypeScript is the canonical implementation; the Go and Rust ports track
it. This file records where the runtimes produce a **different result
for the same input**, and for each whether that is deliberate. Packaging
and API differences are not divergences and live in each runtime's
README.

The engine's own divergences reach every plugin and are recorded once,
in [`parser/DIVERGENCE.md`](https://github.com/tabnas/parser/blob/main/DIVERGENCE.md).
Two of them are visible through hoover and are cited rather than
repeated: error columns after an astral character (2 UTF-16 units in
TypeScript, 1 scalar in Go and Rust) and lone surrogates.

The shared fixtures in `test/spec/*.tsv` are green in all three runtimes.
Every entry below is pinned by an in-language test in the port that
differs, named in the entry, because the TypeScript and Go fixture
runners run every `.tsv` in `test/spec` with the standard
`input`/`expected` header and would refuse a register file there. Adding
`test/spec/divergent.tsv` needs those two runners to learn the register
first.

## Open

### Row after a mapped escape

`parseToEnd` counts rows as it scans, so that an error reported after a
hoovered block carries the right position. The documented rule is that
row and column follow the **source**: an escape consumes two source
characters and the row advances when the escaped source character is a
newline. TypeScript and Go apply that rule in the unknown-escape and
`preserveEscapeChar` branches, and `escapes.tsv` pins both. In the
*mapped* branch they test the replacement instead
(`nl = '\n' === replacement` in `ts/src/hoover.ts`, `replacement == "\n"`
in `go/hoover.go`). The Rust port advances the engine's own cursor over
the consumed source characters, and the engine's `Lexer` has no way to
move its row counter, so Rust follows the source in every branch.

Both inputs below use `escapeChar: "\\"` in a `<...>` block and put a
stray `%` after it; the cells are the position of the `unexpected` error.

| input | escape map | TypeScript | Go | Rust |
|---|---|---|---|---|
| `<a\nb> %` (backslash, letter n) | `{"n": "\n"}` | 2:4 | 2:4 | 1:8 |
| `<a\` newline `b> %` | `{"\n": "N"}` | 1:8 | 1:8 | 2:4 |

The hoovered value is identical in every runtime (`a`, newline, `b` and
`aNb`). Only the position of a later error differs, and in TypeScript
and Go it names a row the source does not have.

The repair is in TypeScript and Go: test the escaped source character in
the mapped branch, as the other two branches already do. The Rust port
then needs no change. Pinned by `row_after_a_mapped_escape_follows_the_source`
in `rs/tests/hoover_test.rs`, which fails when the engine or the port
starts producing the TypeScript rows, so that this entry is deleted
rather than outlived.

### Position of the block token

`hooverMatcher` in `ts/src/hoover.ts` builds the `#HV` token from
`hvpnt` after `parseToEnd` has moved it past the end delimiter, so the
token's own `sI`, `rI` and `cI` name the **end** of the block. Every
matcher the engine ships builds a token at the point where it begins,
and so do both ports: `go/hoover.go` reads the cursor before
`matchStart`, and `rs/src/lib.rs` captures `lexer.point()` before it
advances.

| input | TypeScript | Go | Rust |
|---|---|---|---|
| two spaces, `'''a` newline `b'''` | `sI` 11, 2:5 | `SI` 2, 1:3 | `si` 2, 1:3 |

The value, the position of every later token and the rendered error
messages are identical in all three runtimes; the difference is visible
only to an action that reads the block token's position through
`rule.o0`. The repair is in TypeScript: build the token from the point
captured before `matchStart`, as the engine's own matchers do. The ports
then need no change. Pinned by `the_block_token_is_positioned_at_its_start`
in `rs/tests/hoover_test.rs`, which fails when the port starts producing
the TypeScript position, so that this entry is deleted rather than
outlived.

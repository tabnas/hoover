# Agents Guide — shared spec fixtures

`spec/*.tsv` holds the cross-runtime conformance fixtures. Both runtimes
auto-discover and run **every** file in this directory, so a change here
affects TypeScript and Go together — edit with that in mind.

## Format

Tab-separated, one case per line, with a header row naming the columns.
Blank lines are skipped, and so are comment lines — a line starting with
`#` that contains no tab. (A data row always has at least one tab, so a
`#`-leading source such as a C preprocessor directive still works.)

| Column | Meaning |
|---|---|
| `input` | Source for the test mini-grammar (see ts/test/minigrammar.ts and go/minigrammar_test.go). Escapes `\n` `\r` `\t` `\\` `\uXXXX` are decoded. |
| `expected` | A JSON value (the parse result), or `ERROR` / `ERROR:<position>` for inputs that must fail. Unlike the rest of the fleet the text after the colon is a POSITION — `1:8`, the line and column the rejection is reported at — matched against the rendered message. For a plugin whose job is to consume text up to a delimiter, rejecting at the wrong place is a different defect from rejecting for the wrong reason. A bare `ERROR` accepts any failure. |
| `opts` | Optional JSON object of plugin options (empty means defaults). |

`expected` and `opts` are **not** escape-decoded — they are raw JSON, so
JSON's own escape rules apply (`"a\nb"` is a string containing a newline).
To put a literal backslash in `input`, write `\\`.

`\uXXXX` in `input` takes **exactly four hex digits** and exists so a case
can name a code point that must not appear literally in the file: a NUL
would make git treat the `.tsv` as binary, and a BOM or a non-ASCII space
is invisible in a diff. It is limited to the BMP — TS decodes it to one
UTF-16 code unit and Go to the rune's UTF-8 bytes, which agree below
U+10000 only — so never write a lone surrogate. Anything that is not four
hex digits after `\u` is left alone, so an existing literal `\u` in a
source stays literal.

Results are compared after a JSON round-trip, so key order and the
`OrderedMap` / null-prototype-object representations do not affect the
comparison.

## Who runs what

- TypeScript: `ts/test/parity.test.ts` — `makeRunner(...).dir(...)`.
- Go: `go/parity_test.go` — `support.Runner{...}.Dir(t, dir)`.

Both are short, holding only what is specific to hoover: the mini-grammar
it extends, how to build the parser for a row's options, the position
matching for an `ERROR:` cell, and the `\uXXXX` escape. Everything else —
finding `test/spec`, reading the file, the rest of the escape codec, the
comparison, the `<file>:<line>` in a failure message — comes from
[`@tabnas/support`](https://github.com/tabnas/support) and its Go half, so
the two loaders cannot drift from each other either.

`\uXXXX` is the exception: the shared codec passes `\u` through on
purpose, because a fixture has to be able to carry a literal one, so each
runtime decodes that escape itself over the RAW cell. The two
implementations are kept byte-identical and say so in a comment.

Both discover files by directory listing: adding a `.tsv` here runs it in
both runtimes without touching either runner. An empty fixture, and a spec
directory with no fixtures in it, both **fail** — a runner that reports
green having run nothing is indistinguishable from coverage that was never
there.

## Rules

- Prefer adding a fixture here over a one-off in-language assertion when a
  case is expressible as input → output. That is what keeps the two
  runtimes honest against each other.
- TypeScript is canonical. If the two runtimes disagree, the TS behaviour is
  the expected value — unless Go has exposed a genuine TS defect, in which
  case fix TS first and pin the corrected behaviour here.
- A new fixture must pass in BOTH runtimes: run `go test ./...` (from `go/`)
  and `npm test` (from `ts/`) before considering it done.

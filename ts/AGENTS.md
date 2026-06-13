# Agents Guide — @jsonic/hoover (TypeScript)

This is the **canonical** implementation — the `@jsonic/hoover` npm
package. It is a single [jsonic](https://github.com/jsonicjs/jsonic)
syntax plugin that adds block-delimited string parsing: custom
start/end delimiters, escape handling, and rule-context matching.

jsonic itself is a peer dependency (`jsonic` >= 2); hoover registers a
custom lexer matcher and a `val`-rule alternate on the host instance.

## Layout

- `src/hoover.ts` — the whole plugin. Exports:
  - `Hoover` — the `Plugin` function; register with
    `Jsonic.make().use(Hoover, options)`.
  - `parseToEnd` — the forward scanner, exported for advanced reuse.
  - types `Block`, `HooverOptions`, `ParseResult`, `StartResult`.
- `test/hoover.test.ts` — hand-written unit tests (triplequote, endofline).
- `test/hoover-tsv.test.ts` — runs the shared `../../test/spec/*.tsv`
  fixtures through the same configurations.

The key internals: `buildBlocks` applies defaults (`token` → `#HV`,
`allowUnknownEscape` → `true`, `preserveEscapeChar` → `false`);
`matchStart` checks rule context and the start delimiter; `parseToEnd`
scans to an end delimiter, applying escapes and value resolution.

## Configuration shape

`HooverOptions.block` is an **ordered array** of `Block`, each with a
`name`. Blocks are tried in array order. Do not document or assume a
name-keyed map — the canonical API is an array:

```ts
Jsonic.make().use(Hoover, {
  lex: { order: 4.5e6 },          // optional; default 4.5e6 (before string/number)
  block: [
    { name: 'triplequote', start: { fixed: "'''" }, end: { fixed: "'''" } },
  ],
})
```

## Commands

```bash
npm install
npm run build        # tsc --build src test → dist/ and dist-test/
npm test             # node --test dist-test/*.test.js
npm run test-some --pattern=triplequote
```

Tests run against compiled output in `dist-test/`, so always
`npm run build` after editing `src/` or `test/*.ts`.

## Rules of the road

- Behavior here is the spec: the Go port (`../go/`) must follow. Port in
  the same change, or it falls out of parity with the shared fixtures.
- Prefer a shared fixture in `../test/spec/` (`input → expected-JSON`)
  over a one-off assertion when the case is expressible as input→output;
  wire it into the Go suite too.
- Row/column tracking follows the jsonic engine convention: columns are
  1-based and reset to `1` (not `0`) after a newline.
- The plugin/rule API is jsonic's: `jsonic.token(name)` mints a token,
  `jsonic.rule('val', rs => rs.open({ s: [TOKEN], a }))` prepends the
  hoover alternate, and matchers are registered via
  `jsonic.options({ lex: { match: { hoover: { order, make } } } })`.

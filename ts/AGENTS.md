# Agents Guide — @jsonic/hoover (TypeScript)

This is the **canonical** implementation — the `@jsonic/hoover` npm
package — and the one you change first. It is a single
[tabnas](https://github.com/tabnas/parser) syntax plugin that adds
block-delimited string parsing: custom start/end delimiters, escape
handling, and rule-context matching.

hoover's only dependency is the tabnas engine (`tabnas` >= 2, a peer
dependency). It is grammar-agnostic: it registers a custom lexer matcher
and prepends a `val`-rule alternate on the host instance, so a grammar
defining `val` must already be registered.

## Layout

- `src/hoover.ts` — the whole plugin. Exports:
  - `Hoover` — the `Plugin` function; register with
    `new Tabnas().use(grammar).use(Hoover, options)`.
  - `parseToEnd` — the forward scanner, exported for advanced reuse.
  - types `Block`, `HooverOptions`, `ParseResult`, `StartResult`.
- `test/minigrammar.ts` — a tiny bespoke grammar (`val` + parenthesised
  `group`, **not** JSON) and the `makeMini` helper, so the tests have
  something to plug hoover into without a grammar dependency.
- `test/hoover.test.ts` — behavior tests driving the plugin through that
  grammar.

The key internals: `buildBlocks` applies defaults (`token` → `#HV`,
`allowUnknownEscape` → `true`, `preserveEscapeChar` → `false`);
`matchStart` checks rule context and the start delimiter; `parseToEnd`
scans to an end delimiter, applying escapes and value resolution.

## Configuration shape

`HooverOptions.block` is an **ordered array** of `Block`, each with a
`name`. Blocks are tried in array order. Do not document or assume a
name-keyed map — the canonical API is an array:

```ts
new Tabnas()
  .use(myGrammar)                   // must define the `val` rule
  .use(Hoover, {
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
npm run test-some --pattern="fixed delimiters"
```

Tests run against compiled output in `dist-test/`, so always
`npm run build` after editing `src/` or `test/*.ts`.

## Rules of the road

- Behavior here is the spec: make the change here first, then port to
  the Go port (`../go/`) in the same change. Both ports test against an
  identical tiny grammar with matching cases — add a case to
  `test/hoover.test.ts` and `../go/hoover_test.go` together.
- Row/column tracking follows the tabnas engine convention: columns are
  1-based and reset to `1` (not `0`) after a newline.
- The plugin/rule API is the engine's: `am.token(name)` mints a token,
  `am.rule('val', rs => rs.open({ s: [TOKEN], a }))` prepends the
  hoover alternate, and matchers are registered via
  `am.options({ lex: { match: { hoover: { order, make } } } })`.
- Hoover depends on the host grammar's `val` rule. Register it on a
  grammar-bearing instance; the plugin inspects `am.rule()` up front and
  **throws** a clear error if `val` is missing, instead of silently
  creating an empty rule.
- `escapeChar` may be set without an `escape` map — guard `block.escape`
  before indexing it (a bare escape char with `allowUnknownEscape`
  strips or preserves the char per `preserveEscapeChar`).

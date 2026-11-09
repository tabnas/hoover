/* Copyright (c) 2025 Richard Rodger and other contributors, MIT License */

// Cross-runtime conformance, driven by the shared `test/spec/*.tsv` fixtures
// at the repo root (see ../../test/AGENTS.md).
//
// The fixture loader, the escape codec, the `ERROR:` contract and the row
// loop all come from @tabnas/support, whose Go half `go/parity_test.go`
// and Rust half `rs/tests/parity_test.rs` use to run the SAME files — so
// the three implementations cannot drift without one of them going red,
// and neither can the loaders.
//
// What is left here is only what is specific to hoover: the grammar it
// extends, and what an `ERROR:` cell means.

import { Tabnas } from '@tabnas/parser'
import { findSpecDir, makeRunner } from '@tabnas/support'

import { Hoover } from '../dist/hoover'
import { miniGrammar } from './minigrammar'
import { unescapeHoover } from './unescape'

makeRunner({
  // The runner's own decoding of the input column is bypassed — see
  // unescapeHoover in ./unescape — so the raw cell is read and decoded here.
  parse: (_input, row) => {
    const input = unescapeHoover(row.named('input'))
    // hoover has no grammar of its own: it extends whatever grammar
    // supplies the `val` rule. The tiny local mini-grammar plays that part
    // in every runtime.
    const opts = row.named('opts')
    return new Tabnas()
      .use(miniGrammar)
      .use(Hoover, '' === opts.trim() ? {} : JSON.parse(opts))
      .parse(input)
  },

  // hoover's `ERROR:<want>` cells name a POSITION — `1:8`, the line and
  // column the rejection is reported at — not an error code. That is the
  // thing worth pinning for a plugin whose job is to consume text up to a
  // delimiter: rejecting at the wrong place is a different defect from
  // rejecting for the wrong reason. So it is matched against the rendered
  // message, and a bare `ERROR` still accepts any failure.
  matchError: (err: any, want) => String(err?.message).includes(want),
})
  // `findSpecDir` walks up from this file — `dist-test/` at runtime — to the
  // repo root's `test/spec`, so moving the suite does not mean recounting
  // `..` hops. `dir` then auto-discovers every fixture in it, so adding a
  // .tsv runs it in every runtime without touching any runner.
  .dir(findSpecDir(__dirname))

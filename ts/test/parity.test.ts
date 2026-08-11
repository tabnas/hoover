/* Copyright (c) 2025 Richard Rodger and other contributors, MIT License */

// Cross-runtime conformance, driven by the shared `test/spec/*.tsv` fixtures
// at the repo root (see ../../test/AGENTS.md).
//
// The fixture loader, the escape codec, the `ERROR:` contract and the row
// loop all come from @tabnas/support, whose Go half `go/parity_test.go`
// uses to run the SAME files — so the two implementations cannot drift
// without one of them going red, and neither can the two loaders.
//
// What is left here is only what is specific to hoover: the grammar it
// extends, and what an `ERROR:` cell means.

import { Tabnas } from '@tabnas/parser'
import { findSpecDir, makeRunner } from '@tabnas/support'

import { Hoover } from '../dist/hoover'
import { miniGrammar } from './minigrammar'

const HEX = /^[0-9a-fA-F]{4}$/

// The one thing this repo does not take from @tabnas/support: its own
// escape codec, because hoover's fixtures need a sixth escape.
//
// `\uXXXX` names a code point that must not appear literally in the file:
// a NUL would make git treat the .tsv as binary, and a BOM or a non-ASCII
// space is invisible in a diff. The shared codec passes `\u` through on
// purpose — a fixture has to be able to carry a literal one — so it is
// decoded here, in one pass over the RAW cell. Two passes cannot work:
// after the shared codec, `\u0000` written plainly and `\\u0000` (an
// escaped backslash) are the same six characters.
//
// Non-BMP code points are out of scope: TS decodes to one UTF-16 code unit
// and Go to the rune's UTF-8 bytes, which agree on the BMP only, so a
// fixture must not write a lone surrogate.
//
// Kept byte-identical to specUnescape in go/parity_test.go.
function unescapeHoover(s: string): string {
  if (!s.includes('\\')) return s
  let out = ''
  for (let i = 0; i < s.length; i++) {
    const c = s[i]
    if (c === '\\' && i + 1 < s.length) {
      const n = s[i + 1]
      if (n === 'n') { out += '\n'; i++; continue }
      if (n === 'r') { out += '\r'; i++; continue }
      if (n === 't') { out += '\t'; i++; continue }
      if (n === '\\') { out += '\\'; i++; continue }
      if (n === 'u' && HEX.test(s.substring(i + 2, i + 6))) {
        out += String.fromCharCode(parseInt(s.substring(i + 2, i + 6), 16))
        i += 5
        continue
      }
    }
    out += c
  }
  return out
}

makeRunner({
  // The runner's own decoding of the input column is bypassed — see
  // unescapeHoover above — so the raw cell is read and decoded here.
  parse: (_input, row) => {
    const input = unescapeHoover(row.named('input'))
    // hoover has no grammar of its own: it extends whatever grammar
    // supplies the `val` rule. The tiny local mini-grammar plays that part
    // in both runtimes.
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
  // .tsv runs it in both runtimes without touching either runner.
  .dir(findSpecDir(__dirname))

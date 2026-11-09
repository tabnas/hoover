/* Copyright (c) 2025 Richard Rodger and other contributors, MIT License */

// The shared fixtures' escape codec, as `parity.test.ts` and
// `window.test.ts` read the input column.

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
export function unescapeHoover(s: string): string {
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

/* Copyright (c) 2026 Richard Rodger and other contributors, MIT License */

//! What the fixture runners share: the input column's codec, and a row's
//! options. `tests/parity_test.rs` holds the parse to each row, and
//! `tests/reader_test.rs` a parse from a reader to the parse of the same
//! text; both read the rows through here.

use tabnas_hoover::HooverOptions;
use tabnas_support::Row;

/// Decode a fixture input. The one thing this repo does not take from
/// tabnas-support: its own escape codec, because hoover's fixtures need
/// a sixth escape.
///
/// `\uXXXX` names a code point that must not appear literally in the
/// file: a NUL would make git treat the .tsv as binary, and a BOM or a
/// non-ASCII space is invisible in a diff. The shared codec passes `\u`
/// through on purpose (a fixture has to be able to carry a literal one),
/// so it is decoded here, in one pass over the RAW cell. Two passes
/// cannot work: after the shared codec, `\u0000` written plainly and
/// `\\u0000` (an escaped backslash) are the same six characters.
///
/// Non-BMP code points are out of scope: TypeScript decodes to one
/// UTF-16 code unit and Go to the rune's UTF-8 bytes, which agree on the
/// BMP only, so a fixture must not write a lone surrogate.
///
/// Kept byte-identical in effect to `unescapeHoover` in
/// ts/test/unescape.ts and `specUnescape` in go/parity_test.go.
pub fn spec_unescape(text: &str) -> String {
    if !text.contains('\\') {
        return text.to_string();
    }
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::with_capacity(text.len());
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c == '\\' && i + 1 < chars.len() {
            match chars[i + 1] {
                'n' => {
                    out.push('\n');
                    i += 2;
                    continue;
                }
                'r' => {
                    out.push('\r');
                    i += 2;
                    continue;
                }
                't' => {
                    out.push('\t');
                    i += 2;
                    continue;
                }
                '\\' => {
                    out.push('\\');
                    i += 2;
                    continue;
                }
                'u' => {
                    let hex: String = chars[i + 2..].iter().take(4).collect();
                    if hex.len() == 4 && hex.chars().all(|h| h.is_ascii_hexdigit()) {
                        let code = u32::from_str_radix(&hex, 16).expect("four hex digits");
                        out.push(char::from_u32(code).expect("a BMP code point, not a surrogate"));
                        i += 6;
                        continue;
                    }
                }
                _ => {}
            }
        }
        out.push(c);
        i += 1;
    }
    out
}

/// A row's `opts`, as hoover's options. Malformed options are a defect in
/// the fixture, so they panic rather than becoming a failure that an
/// `ERROR` row would count as a pass.
pub fn options_for(row: &Row) -> HooverOptions {
    let opts = row.named("opts");
    if opts.trim().is_empty() {
        return HooverOptions::default();
    }
    let document: serde_json::Value = serde_json::from_str(opts)
        .unwrap_or_else(|error| panic!("{}: opts is not JSON: {error}", row.location()));
    HooverOptions::from_json(&document)
        .unwrap_or_else(|error| panic!("{}: {error}", row.location()))
}

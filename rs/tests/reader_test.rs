/* Copyright (c) 2026 Richard Rodger and other contributors, MIT License */

//! A parse from a reader (`Tabnas::parse_reader`, admin ADR-30 and ADR-33).
//!
//! hoover is a plugin other grammars install beneath matchers of their
//! own, so it declares no window: the grammar that builds a parser whole
//! declares `lex.window` once every matcher it installs, hoover's among
//! them, reads the source only through the window. hoover's matcher does:
//! a start delimiter the window ends inside, or a block whose scan reaches
//! the window's end, is read again over a window asked for twice as much,
//! and the end of the window is the end of the source only once the source
//! has nothing more to give.
//!
//! These tests declare the window on the mini grammar's parser, as a host
//! would on its own, and hold hoover to it: every row of the shared
//! fixtures, and blocks of every shape cut at every place, parse the same
//! from a reader, in pieces of 1, 3 and 64 bytes, under windows of a few
//! characters and the engine's own, as from their text: the same value, or
//! the same error code, position, row, column and token text. Blocks longer
//! than the engine's window, and a long block read under a window of 256
//! characters that moves on many times, parse the same too.

mod common;

use std::io::Read;
use std::path::Path;

use tabnas::{Tabnas, WindowSizes};
use tabnas_hoover::HooverOptions;
use tabnas_support::{find_spec_dir, load_spec_dir, SpecOptions};

use common::mini_grammar::make_mini;
use common::spec::{options_for, spec_unescape};

/// A reader that hands its bytes over at most `size` at a time.
struct Pieces<'a> {
    bytes: &'a [u8],
    at: usize,
    size: usize,
}

impl<'a> Pieces<'a> {
    fn new(bytes: &'a [u8], size: usize) -> Self {
        Pieces { bytes, at: 0, size }
    }
}

impl Read for Pieces<'_> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let n = self.size.min(buf.len()).min(self.bytes.len() - self.at);
        buf[..n].copy_from_slice(&self.bytes[self.at..self.at + n]);
        self.at += n;
        Ok(n)
    }
}

/// The mini grammar with hoover installed, and `lex.window` declared on the
/// parser, as a host declares it on the parser it makes whole.
fn windowed(options: HooverOptions) -> Tabnas {
    let mut parser = make_mini(options);
    parser
        .set_options(|options| options.lex.window = true)
        .expect("lex.window sets");
    parser
}

/// hoover's options from the JSON shape the fixtures write.
fn options(json: &str) -> HooverOptions {
    let document: serde_json::Value = serde_json::from_str(json).expect("options are JSON");
    HooverOptions::from_json(&document).expect("options are hoover's")
}

/// What a parse shows: its value, or its error's code, position, row,
/// column and token text. An error's excerpt is left out: a parse from a
/// reader shows a long line only as far back as the window reaches
/// (tabnas/parser `DIVERGENCE.md`), the one difference allowed.
fn outcome(parsed: Result<tabnas::Value, tabnas::TabnasError>) -> String {
    match parsed {
        Ok(value) => format!("ok {}", value.to_json()),
        Err(e) => format!(
            "err {} at {} {}:{} {:?}",
            e.code, e.pos, e.row, e.col, e.src
        ),
    }
}

/// The first `n` bytes of `s`, cut back to a character boundary.
fn cut(s: &str, n: usize) -> &str {
    let mut end = n.min(s.len());
    while !s.is_char_boundary(end) {
        end -= 1;
    }
    &s[..end]
}

/// The pieces a document is handed over in, and the windows (margin and
/// block, in characters) it is read under besides the engine's own: a few
/// characters, so that every block meets the window's edge.
const PIECES: [usize; 3] = [1, 3, 64];
const WINDOWS: [(usize, usize); 3] = [(1, 1), (3, 2), (64, 64)];

/// The reads that differ from the whole text's parse, described.
fn differences(at: &str, text: &str, whole: &str, reads: &[(String, String)]) -> Vec<String> {
    reads
        .iter()
        .filter(|(_, read)| read != whole)
        .map(|(how, read)| {
            format!(
                "{at}: {:?} from a reader in {how}\n  whole:  {}\n  reader: {}",
                cut(text, 80),
                cut(whole, 300),
                cut(read, 300)
            )
        })
        .collect()
}

/// `text` parsed by `parser` whole, and from a reader in each of [`PIECES`]
/// under the engine's window and each of [`WINDOWS`]: how many reader
/// parses were compared, and where they differ.
fn same_from_reader(parser: &Tabnas, at: &str, text: &str) -> (usize, Vec<String>) {
    let whole = outcome(parser.parse(text));
    let bytes = text.as_bytes();
    let mut reads: Vec<(String, String)> = Vec::new();
    for size in PIECES {
        reads.push((
            format!("pieces of {size}"),
            outcome(parser.parse_reader(Pieces::new(bytes, size))),
        ));
        for (margin, block) in WINDOWS {
            let sizes = WindowSizes {
                margin,
                block,
                chunk: size,
            };
            reads.push((
                format!("pieces of {size}, margin {margin}, block {block}"),
                outcome(parser.parse_reader_with_sizes(Pieces::new(bytes, size), sizes)),
            ));
        }
    }
    (reads.len(), differences(at, text, &whole, &reads))
}

#[test]
fn hoover_declares_no_window() {
    let parser = make_mini(options(
        r#"{"block":[{"name":"a","start":{"fixed":"<"},"end":{"fixed":">"}}]}"#,
    ));
    assert!(!parser.options.lex.window);
}

/// Every row of the shared fixtures parses the same from a reader as from
/// its text, on a parser that declares the window.
#[test]
fn every_row_parses_the_same_from_a_reader() {
    let dir = find_spec_dir(Some(Path::new(env!("CARGO_MANIFEST_DIR")))).expect("test/spec");
    let specs = load_spec_dir(&dir, &SpecOptions::default()).unwrap_or_else(|e| panic!("{e}"));
    let mut compared = 0;
    let mut failures = Vec::new();
    for spec in &specs {
        for row in &spec.rows {
            let input = spec_unescape(row.named("input"));
            let (count, found) =
                same_from_reader(&windowed(options_for(row)), &row.location(), &input);
            compared += count;
            failures.extend(found);
        }
    }
    eprintln!("reader fixtures: {compared} reader parses (100%)");
    assert!(compared > 600, "{compared}");
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// The block shapes the fixtures hold, and the ones ini installs, each
/// with documents that put a delimiter, an escape or the end of the block
/// at every distance from a window of a few characters. Kept in step with
/// `SHAPES` in ts/test/window.test.ts and `shapes` in go/window_test.go.
fn shapes() -> Vec<(&'static str, Vec<&'static str>)> {
    vec![
        (
            r#"{"block":[{"name":"tq","start":{"fixed":"'''"},"end":{"fixed":"'''"}}]}"#,
            vec![
                "'''x'''",
                "'''a b\nc''d'''",
                "(''' x ''')",
                "'''open",
                "''x",
            ],
        ),
        (
            r#"{"block":[{"name":"line","start":{"fixed":"~"},"end":{"fixed":["\r\n","\n",""],"consume":["\r\n","\n"]}}]}"#,
            vec!["~a b\r\n", "~a b\n", "~a b", "~a\rb\r\n", "~a\r", "~\r\n"],
        ),
        (
            r#"{"block":[{"name":"angle","start":{"fixed":"<"},"end":{"fixed":">"},"escapeChar":"\\","escape":{"n":"\n",">":">","\\":"\\"},"allowUnknownEscape":false}]}"#,
            vec!["<a\\>b\\\\c\\nd>", "<a\\", "<a\\zb>", "<abc", "(<x>)"],
        ),
        // A block whose start delimiter begins another block's, tried
        // first, and an end delimiter that holds another one, kept: a
        // window that ends inside either must read on before the block
        // decides, or the shorter delimiter wins.
        (
            r#"{"block":[{"name":"two","start":{"fixed":"~~"},"end":{"fixed":"!"}},{"name":"one","start":{"fixed":"~"},"end":{"fixed":"!"}}]}"#,
            vec!["~~a!", "~a!", "~~!", "~~", "~"],
        ),
        (
            r#"{"block":[{"name":"tail","start":{"fixed":"<"},"end":{"fixed":["abc","b"],"consume":false}}]}"#,
            vec!["<xabc", "<xab", "<xb", "<xa"],
        ),
        (
            r#"{"block":[{"name":"eol","start":{"rule":{"state":""}},"end":{"fixed":["\n","\r\n",";",""],"consume":["\n","\r\n"]},"escapeChar":"\\","escape":{";":";","\\":"\\"},"allowUnknownEscape":true,"preserveEscapeChar":true,"trim":true}]}"#,
            vec![
                "  a b \\; c \\x d  \r\n",
                "a\\",
                "a;",
                "\u{feff} trimmed \u{a0}",
                "x\u{1d11e}y\r\n",
            ],
        ),
    ]
}

/// Each block shape, cut by a window of a few characters at every place:
/// a start delimiter, an end delimiter of one character and of two, an
/// escape and the character it escapes, the end of the source, and the
/// errors, an unterminated block and a rejected escape.
#[test]
fn every_block_meets_the_window_edge() {
    let mut compared = 0;
    let mut failures = Vec::new();
    for (json, documents) in shapes() {
        let parser = windowed(options(json));
        for document in documents {
            let (count, found) = same_from_reader(&parser, json, document);
            compared += count;
            failures.extend(found);
        }
    }
    assert!(compared > 200, "{compared}");
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// Blocks longer than the engine's window, which reaches 64 KiB past the
/// cursor before each token, parse the same from a reader as from their
/// text: a block ended by each kind of delimiter, one full of escapes, and
/// an unterminated one.
#[test]
fn blocks_longer_than_the_window_parse_the_same_from_a_reader() {
    let long = 64 * 1024 + 2048;
    let cases = [
        (
            shapes()[0].0,
            vec![
                format!("'''{}'''", "t".repeat(long)),
                format!("'''{}", "t".repeat(long)),
            ],
        ),
        (
            shapes()[1].0,
            vec![
                format!("~{}\r\n", "l".repeat(long)),
                format!("~{}", "l".repeat(long)),
                format!("~{}\r", "l\r".repeat(long / 2)),
            ],
        ),
        (
            shapes()[2].0,
            vec![
                format!("<{}>", "a\\>".repeat(long / 3)),
                format!("<{}\\", "a".repeat(long)),
                format!("<{}", "a".repeat(long)),
            ],
        ),
        (
            shapes()[5].0,
            vec![format!("{}\u{1d11e}\r\n", "x\\; \u{e9}".repeat(long / 5))],
        ),
    ];
    let mut failures = Vec::new();
    for (json, documents) in cases {
        let parser = windowed(options(json));
        for (n, document) in documents.iter().enumerate() {
            let whole = outcome(parser.parse(document));
            let reads: Vec<(String, String)> = [4096, 64 * 1024 + 3]
                .iter()
                .map(|&size| {
                    (
                        format!("pieces of {size}"),
                        outcome(parser.parse_reader(Pieces::new(document.as_bytes(), size))),
                    )
                })
                .collect();
            failures.extend(differences(
                &format!("{json} document {n}"),
                document,
                &whole,
                &reads,
            ));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// A long block, under the engine's window and under one of 256
/// characters that moves on many times before its end, parses the same
/// from a reader as from its text; so does the same block left open,
/// which the reader rejects where the whole text does.
#[test]
fn a_long_block_parses_the_same_from_a_reader() {
    let mut body = String::new();
    for i in 0..20_000 {
        body.push_str(&format!("row {i} \\> \u{e9}\\n"));
    }
    let text = format!("(<{body}>)");
    let open = format!("(<{body}");
    let parser = windowed(options(shapes()[2].0));
    let small = WindowSizes {
        margin: 256,
        block: 256,
        chunk: 4096,
    };
    let mut failures = Vec::new();
    for (n, document) in [&text, &open].into_iter().enumerate() {
        let whole = outcome(parser.parse(document));
        assert_eq!(whole.starts_with("ok "), 0 == n, "{}", cut(&whole, 200));
        let reads = vec![
            (
                "pieces of 4096".to_string(),
                outcome(parser.parse_reader(Pieces::new(document.as_bytes(), 4096))),
            ),
            (
                "pieces of 4096, margin 256, block 256".to_string(),
                outcome(
                    parser.parse_reader_with_sizes(Pieces::new(document.as_bytes(), 4096), small),
                ),
            ),
        ];
        failures.extend(differences(
            &format!("document {n}"),
            document,
            &whole,
            &reads,
        ));
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

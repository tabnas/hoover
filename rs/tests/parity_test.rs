/* Copyright (c) 2025 Richard Rodger and other contributors, MIT License */

// Cross-runtime conformance, driven by the shared `test/spec/*.tsv`
// fixtures at the repo root (see ../../test/AGENTS.md).
//
// The fixture loader, the escape codec, the `ERROR:` contract and the row
// loop all come from tabnas-support, whose TypeScript and Go halves
// `ts/test/parity.test.ts` and `go/parity_test.go` use to run the SAME
// files, so the three implementations cannot drift without one of them
// going red, and neither can the three loaders.
//
// What is left here is only what is specific to hoover: the grammar it
// extends, how a row's options build the parser, and what an `ERROR:`
// cell means.

mod common;

use std::path::Path;

use common::mini_grammar::make_mini;
use common::spec::{options_for, spec_unescape};
use tabnas_support::{find_spec_dir, Failure, Row, Runner, Value};

/// Build the parser for one row: the mini grammar (hoover has no grammar
/// of its own) plus hoover with the row's `opts`, as the other two
/// runners do.
fn parser_for(row: &Row) -> tabnas::Tabnas {
    make_mini(options_for(row))
}

fn runner() -> Runner {
    Runner::new_with_row(|_input, row| {
        // The runner's own decoding of the input column is bypassed (see
        // spec_unescape) so the raw cell is read and decoded here.
        let input = spec_unescape(row.named("input"));
        parser_for(row)
            .parse(&input)
            .map(|value| Value::from(value.to_json()))
            .map_err(|error| {
                let failure = Failure::new(error.code.clone()).with_message(error.to_string());
                if error.row > 0 && error.col > 0 {
                    failure.at(error.row, error.col)
                } else {
                    failure
                }
            })
    })
    // hoover's `ERROR:<want>` cells name a POSITION, `1:8`, the line and
    // column the rejection is reported at, not an error code. That is
    // the thing worth pinning for a plugin whose job is to consume text
    // up to a delimiter: rejecting at the wrong place is a different
    // defect from rejecting for the wrong reason. So it is compared with
    // the failure's own row and column, exactly, and a bare `ERROR` still
    // accepts any failure.
    .match_error(|failure, want, _row| match position(want) {
        Some((row, col)) => failure.row == Some(row) && failure.col == Some(col),
        None => failure.message.contains(want),
    })
}

/// `1:8` as a (row, column) pair; anything else is not a position.
fn position(want: &str) -> Option<(usize, usize)> {
    let (row, col) = want.split_once(':')?;
    Some((row.parse().ok()?, col.parse().ok()?))
}

/// Every fixture in the spec directory. `find_spec_dir` walks up from
/// the crate directory, and `dir` discovers the files by listing, so
/// adding a .tsv runs it in every runtime without touching any runner.
/// An empty directory, and an empty fixture, both fail inside the runner.
#[test]
fn spec() {
    let dir = find_spec_dir(Some(Path::new(env!("CARGO_MANIFEST_DIR")))).expect("test/spec");
    runner().dir(&dir);
}

/// The census this suite is expected to cover. A fixture renamed or
/// removed would otherwise be a silent loss of coverage: `dir` runs what
/// it finds, and finding less is not a failure to it.
#[test]
fn every_fixture_is_present() {
    let dir = find_spec_dir(Some(Path::new(env!("CARGO_MANIFEST_DIR")))).expect("test/spec");
    for name in [
        "end-delimiters.tsv",
        "escapes.tsv",
        "fixed-delimiters.tsv",
        "rule-context.tsv",
        "start-delimiters.tsv",
        "trim.tsv",
        "values.tsv",
    ] {
        assert!(dir.join(name).is_file(), "missing fixture {name}");
    }
}

/// The sixth escape, in isolation: exactly four hex digits, BMP only,
/// and anything else left alone so a literal `\u` in a source survives.
#[test]
fn spec_unescape_decodes_the_hoover_escapes() {
    assert_eq!(spec_unescape("a\\u0000b"), "a\u{0}b");
    assert_eq!(spec_unescape("<\\ufeffhello>"), "<\u{FEFF}hello>");
    assert_eq!(spec_unescape("\\n\\t\\r\\\\"), "\n\t\r\\");
    assert_eq!(spec_unescape("\\u00zz"), "\\u00zz");
    assert_eq!(spec_unescape("\\u12"), "\\u12");
    assert_eq!(spec_unescape("plain"), "plain");
}

/// The position channel has to be seen to bite: a row that pins the wrong
/// row and column must fail, or `ERROR:1:8` asserts nothing more than a
/// bare `ERROR` would.
#[test]
#[should_panic(expected = "does not match")]
fn a_wrong_error_position_fails_the_row() {
    let spec = tabnas_support::parse_spec(
        "probe.tsv",
        "input\texpected\topts\n<a\\\\zb> %\tERROR:1:9\t{\"block\":[{\"name\":\"angle\",\"start\":{\"fixed\":\"<\"},\"end\":{\"fixed\":\">\"},\"escapeChar\":\"\\\\\"}]}\n",
        &tabnas_support::SpecOptions::default(),
    )
    .expect("the probe fixture loads");
    runner().spec(&spec);
}

/// The same probe at the right position passes, so the guard above fails
/// for the reason it claims.
#[test]
fn the_right_error_position_passes_the_row() {
    let spec = tabnas_support::parse_spec(
        "probe.tsv",
        "input\texpected\topts\n<a\\\\zb> %\tERROR:1:8\t{\"block\":[{\"name\":\"angle\",\"start\":{\"fixed\":\"<\"},\"end\":{\"fixed\":\">\"},\"escapeChar\":\"\\\\\"}]}\n",
        &tabnas_support::SpecOptions::default(),
    )
    .expect("the probe fixture loads");
    runner().spec(&spec);
}

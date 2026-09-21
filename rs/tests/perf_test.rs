/* Copyright (c) 2021-2026 Richard Rodger and other contributors, MIT License */

// Guards against a performance regression where a caller (or a future
// convenience wrapper) rebuilds the engine + grammar + hoover plugin on
// EVERY parse instead of building one instance and reusing it. Building
// the grammar dominates a parse, so a rebuild-per-call path is many
// times slower than reusing one instance.
//
// Hoover is a PLUGIN, not a crate with a public `parse(src)` convenience:
// the caller owns the instance (`Tabnas::new()`, the host grammar, then
// `hoover`). There is therefore nothing for the crate to cache. Instead
// this test pins the representative usage contract: a single construction
// is amortised over N parses, so parsing N times on ONE instance must be
// far cheaper than rebuilding the instance for each of those N parses.
//
// The check is machine-INDEPENDENT: it compares rebuild-per-parse against
// instance reuse on the SAME machine in the SAME run, so a slow CI box
// cannot make it flaky (both sides scale together). There is deliberately
// NO wall-clock budget. Mirrors ts/test/perf.test.ts and go/perf_test.go.

mod common;

use std::time::Instant;

use common::mini_grammar::make_mini;
use tabnas::{Tabnas, Value};
use tabnas_hoover::{Block, HooverOptions};

#[test]
fn reusing_one_instance_is_far_cheaper_than_rebuilding_per_parse() {
    let src = "'''hello world'''";
    let n = 2000;
    let want = Value::String("hello world".to_string());

    // build constructs a fresh hoover-enabled instance, the work a
    // rebuild-per-parse path repeats on every call.
    let build = || -> Tabnas {
        make_mini(HooverOptions::new(vec![Block::delimited(
            "triplequote",
            "'''",
            "'''",
        )]))
    };

    // Warm both paths so the comparison is steady-state.
    for _ in 0..50 {
        assert_eq!(build().parse(src).expect("warm rebuild parse"), want);
    }
    let reused = build();
    for _ in 0..50 {
        assert_eq!(reused.parse(src).expect("warm reuse parse"), want);
    }

    // Rebuild the instance for every parse (the regression shape).
    let started = Instant::now();
    for _ in 0..n {
        assert_eq!(build().parse(src).expect("rebuild parse"), want);
    }
    let rebuild = started.elapsed();

    // Parse the same N times reusing one instance (the correct shape).
    let started = Instant::now();
    for _ in 0..n {
        assert_eq!(reused.parse(src).expect("reuse parse"), want);
    }
    let reuse = started.elapsed();

    // Reusing one instance must be much cheaper than rebuilding per
    // parse: building the grammar dominates. If someone makes the hot
    // path rebuild per parse, the two converge and this trips. Reuse must
    // be at least 4x faster (rebuild > 4*reuse); on this tiny grammar the
    // real gap is far larger, so the slack absorbs scheduling noise
    // without an absolute wall-clock budget.
    let ratio = rebuild.as_secs_f64() / reuse.as_secs_f64();
    assert!(
        rebuild > reuse * 4,
        "instance reuse is not meaningfully faster than rebuilding per parse: \
         {n} rebuild-per-parse took {rebuild:?} vs {reuse:?} reusing one instance \
         (ratio {ratio:.1}x, want > 4x). A parse hot path appears to rebuild the \
         engine/grammar each call; build one instance and reuse it."
    );
    eprintln!("rebuild-per-parse={rebuild:?} reuse={reuse:?} ratio={ratio:.2}x");
}

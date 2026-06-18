/* Copyright (c) 2021-2026 Richard Rodger and other contributors, MIT License */

package tabnashoover

import (
	"testing"
	"time"

	tabnas "github.com/tabnas/parser/go"
)

// TestParseReusesInstance guards against a performance regression where a
// caller (or a future convenience wrapper) rebuilds the engine + grammar +
// hoover plugin on EVERY parse instead of building one instance and reusing
// it. Building the grammar dominates a parse, so a rebuild-per-call path is
// many times slower than reusing one instance.
//
// Hoover is a PLUGIN, not a package with a public Parse(src) convenience: the
// caller owns the instance (Make -> Use(miniGrammar) -> UseDefaults(Hoover)).
// There is therefore nothing for the package to cache. Instead this test
// pins the representative usage contract: a single Make/Use is amortised over
// N parses, so parsing N times on ONE instance must be far cheaper than
// rebuilding the instance for each of those N parses.
//
// The check is machine-INDEPENDENT: it compares rebuild-per-parse against
// instance reuse on the SAME machine in the SAME run, so a slow CI box cannot
// make it flaky (both sides scale together). There is deliberately NO
// wall-clock budget.
func TestParseReusesInstance(t *testing.T) {
	const src = `'''hello world'''`
	const n = 2000

	opts := map[string]any{
		"block": []*Block{{
			Name:  "triplequote",
			Start: StartSpec{Fixed: []string{"'''"}},
			End:   EndSpec{Fixed: []string{"'''"}},
		}},
	}

	// build constructs a fresh hoover-enabled instance — the work a
	// rebuild-per-parse path repeats on every call.
	build := func() *tabnas.Tabnas {
		j := tabnas.Make()
		if err := j.Use(miniGrammar); err != nil {
			t.Fatalf("miniGrammar: %v", err)
		}
		if err := j.UseDefaults(Hoover, Defaults, opts); err != nil {
			t.Fatalf("hoover Use: %v", err)
		}
		return j
	}

	// Warm both paths so the comparison is steady-state.
	for i := 0; i < 50; i++ {
		j := build()
		if _, err := j.Parse(src); err != nil {
			t.Fatalf("warm rebuild parse error: %v", err)
		}
	}
	reused := build()
	for i := 0; i < 50; i++ {
		if _, err := reused.Parse(src); err != nil {
			t.Fatalf("warm reuse parse error: %v", err)
		}
	}

	// Rebuild the instance for every parse (the regression shape).
	t0 := time.Now()
	for i := 0; i < n; i++ {
		j := build()
		if _, err := j.Parse(src); err != nil {
			t.Fatalf("rebuild parse error: %v", err)
		}
	}
	rebuild := time.Since(t0)

	// Parse the same N times reusing one instance (the correct shape).
	t1 := time.Now()
	for i := 0; i < n; i++ {
		if _, err := reused.Parse(src); err != nil {
			t.Fatalf("reuse parse error: %v", err)
		}
	}
	reuse := time.Since(t1)

	// Reusing one instance must be much cheaper than rebuilding per parse —
	// building the grammar dominates. If someone makes the hot path rebuild
	// per parse, the two converge and this trips. We assert reuse is at least
	// 4x faster (rebuild > 4*reuse); on this tiny grammar the real gap is far
	// larger, so the slack absorbs scheduling noise without an absolute
	// wall-clock budget.
	if rebuild <= 4*reuse {
		t.Errorf("instance reuse is not meaningfully faster than rebuilding "+
			"per parse: %d rebuild-per-parse took %v vs %v reusing one instance "+
			"(ratio %.1fx, want > 4x). A parse hot path appears to rebuild the "+
			"engine/grammar each call; build one instance (Make/Use) and reuse it.",
			n, rebuild, reuse, float64(rebuild)/float64(reuse))
	}
	t.Logf("rebuild-per-parse=%v  reuse=%v  ratio=%.2fx",
		rebuild, reuse, float64(rebuild)/float64(reuse))
}

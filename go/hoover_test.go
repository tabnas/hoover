/* Copyright (c) 2021-2026 Richard Rodger and other contributors, MIT License */

package tabnashoover

import (
	"reflect"
	"testing"

	tabnas "github.com/tabnas/parser/go"
)

// These tests run the hoover plugin against the tiny local grammar in
// minigrammar_test.go (val + parenthesised group). The grammar exists only
// to give hoover something to plug into; hoover's only production
// dependency is the tabnas engine.

func parseEq(t *testing.T, j *tabnas.Tabnas, src string, want any) {
	t.Helper()
	got, err := j.Parse(src)
	if err != nil {
		t.Fatalf("Parse(%q) error: %v", src, err)
	}
	if !reflect.DeepEqual(got, want) {
		t.Errorf("Parse(%q) = %#v, want %#v", src, got, want)
	}
}

func parseErr(t *testing.T, j *tabnas.Tabnas, src string) {
	t.Helper()
	if got, err := j.Parse(src); err == nil {
		t.Errorf("Parse(%q) = %#v, want an error", src, got)
	}
}

// tripleQuote: ”'...”' fixed delimiters, default options.
func tripleQuote(t *testing.T) *tabnas.Tabnas {
	return makeMini(t, map[string]any{
		"block": []*Block{{
			Name:  "triplequote",
			Start: StartSpec{Fixed: []string{"'''"}},
			End:   EndSpec{Fixed: []string{"'''"}},
		}},
	})
}

func TestFixedDelimiters(t *testing.T) {
	j := tripleQuote(t)
	parseEq(t, j, `'''x'''`, "x")
	parseEq(t, j, `'''hello world'''`, "hello world") // internal spaces preserved
	parseEq(t, j, "'''a\nb'''", "a\nb")               // newlines preserved
	parseEq(t, j, "'''  spaced  '''", "  spaced  ")   // no trim by default
	parseEq(t, j, `('''x''')`, "x")                   // nested in a group
	parseEq(t, j, `(''' a b ''')`, " a b ")
}

func TestEOFAndMultipleEndDelimiters(t *testing.T) {
	// ~ opens; closes on '>', '!' or end-of-input.
	j := makeMini(t, map[string]any{
		"block": []*Block{{
			Name:  "tilde",
			Start: StartSpec{Fixed: []string{"~"}},
			End:   EndSpec{Fixed: []string{">", "!", ""}},
		}},
	})
	parseEq(t, j, `~hello world`, "hello world") // EOF terminates
	parseEq(t, j, `~a>`, "a")                    // first delimiter
	parseEq(t, j, `~a!`, "a")                    // second delimiter
}

func TestEscapes(t *testing.T) {
	// < ... > with backslash escapes; allowUnknownEscape defaults to true.
	j := makeMini(t, map[string]any{
		"block": []*Block{{
			Name:       "angle",
			Start:      StartSpec{Fixed: []string{"<"}},
			End:        EndSpec{Fixed: []string{">"}},
			EscapeChar: "\\",
			Escape:     map[string]string{"n": "\n", ">": ">", "\\": "\\"},
		}},
	})
	parseEq(t, j, `<a\>b>`, "a>b")    // escaped end delimiter
	parseEq(t, j, `<a\nb>`, "a\nb")   // mapped escape
	parseEq(t, j, `<a\\b>`, "a\\b")   // escaped backslash
	parseEq(t, j, `<a\zb>`, "azb")    // unknown escape: backslash dropped
	parseEq(t, j, "<a\\\nb>", "a\nb") // backslash before a literal newline
}

func TestRejectUnknownEscape(t *testing.T) {
	f := false
	j := makeMini(t, map[string]any{
		"block": []*Block{{
			Name:               "angle",
			Start:              StartSpec{Fixed: []string{"<"}},
			End:                EndSpec{Fixed: []string{">"}},
			EscapeChar:         "\\",
			Escape:             map[string]string{">": ">"},
			AllowUnknownEscape: &f,
		}},
	})
	parseEq(t, j, `<a\>b>`, "a>b") // known escape still works
	parseErr(t, j, `<a\zb>`)       // unknown escape rejected
}

func TestPreserveEscapeChar(t *testing.T) {
	j := makeMini(t, map[string]any{
		"block": []*Block{{
			Name:               "angle",
			Start:              StartSpec{Fixed: []string{"<"}},
			End:                EndSpec{Fixed: []string{">"}},
			EscapeChar:         "\\",
			PreserveEscapeChar: true,
		}},
	})
	parseEq(t, j, `<a\zb>`, "a\\zb") // escape char kept in output
}

func TestTrim(t *testing.T) {
	j := makeMini(t, map[string]any{
		"block": []*Block{{
			Name:  "angle",
			Start: StartSpec{Fixed: []string{"<"}},
			End:   EndSpec{Fixed: []string{">"}},
			Trim:  true,
		}},
	})
	parseEq(t, j, `<  hello  >`, "hello")
	parseEq(t, j, `< a b >`, "a b") // internal spaces kept, edges trimmed
}

func TestSelectiveEndConsume(t *testing.T) {
	// Closes on ';' or end-of-input; only ';' is consumed.
	j := makeMini(t, map[string]any{
		"block": []*Block{{
			Name:  "tilde",
			Start: StartSpec{Fixed: []string{"~"}},
			End:   EndSpec{Fixed: []string{";", ""}, Consume: []string{";"}},
		}},
	})
	parseEq(t, j, `~a b;`, "a b")
	parseEq(t, j, `~a b`, "a b")
}

func TestRuleContextParent(t *testing.T) {
	// The @ block only matches when the current rule's parent is `group`.
	j := makeMini(t, map[string]any{
		"block": []*Block{{
			Name: "at",
			Start: StartSpec{
				Fixed: []string{"@"},
				Rule: &HooverRuleSpec{
					Parent: &HooverRuleFilter{Include: []string{"group"}},
				},
			},
			End: EndSpec{Fixed: []string{"@"}},
		}},
	})
	// Inside a group (parent == group): matches.
	parseEq(t, j, `(@hello world@)`, "hello world")
	// At top level (parent is not group): does not match, so the bare
	// '@' is unexpected and the parse fails.
	parseErr(t, j, `@hello world@`)
}

func TestCustomTokenName(t *testing.T) {
	// A non-default token name still parses and is registered on the engine.
	j := tabnas.Make()
	if err := j.Use(miniGrammar); err != nil {
		t.Fatalf("miniGrammar: %v", err)
	}
	if err := j.UseDefaults(Hoover, Defaults, map[string]any{
		"block": []*Block{{
			Name:  "tq",
			Token: "#XX",
			Start: StartSpec{Fixed: []string{"'''"}},
			End:   EndSpec{Fixed: []string{"'''"}},
		}},
	}); err != nil {
		t.Fatalf("hoover Use: %v", err)
	}
	parseEq(t, j, `'''x'''`, "x")
	if _, ok := j.RSM()["val"]; !ok {
		t.Fatal("val rule missing")
	}
}

func TestFailFastMissingGrammar(t *testing.T) {
	// Registering hoover on a bare engine (no grammar, no `val` rule)
	// returns a clear error instead of failing confusingly later.
	j := tabnas.Make()
	err := j.UseDefaults(Hoover, Defaults, map[string]any{
		"block": []*Block{{
			Name:  "tq",
			Start: StartSpec{Fixed: []string{"'''"}},
			End:   EndSpec{Fixed: []string{"'''"}},
		}},
	})
	if err == nil {
		t.Fatal("expected an error registering hoover without a grammar")
	}
}

func TestStartConsume(t *testing.T) {
	// consume: false leaves the start delimiter in the value.
	j1 := makeMini(t, map[string]any{
		"block": []*Block{{
			Name:  "a",
			Start: StartSpec{Fixed: []string{"<"}, Consume: false},
			End:   EndSpec{Fixed: []string{">"}},
		}},
	})
	parseEq(t, j1, `<hi>`, "<hi")

	// consume: [...] consumes only the listed start delimiters.
	j2 := makeMini(t, map[string]any{
		"block": []*Block{{
			Name:  "a",
			Start: StartSpec{Fixed: []string{"<", "~"}, Consume: []string{"<"}},
			End:   EndSpec{Fixed: []string{">"}},
		}},
	})
	parseEq(t, j2, `<hi>`, "hi")  // '<' consumed
	parseEq(t, j2, `~hi>`, "~hi") // '~' kept
}

func TestEndConsumeBool(t *testing.T) {
	// consume: false leaves the end delimiter — here the group's ')'.
	j1 := makeMini(t, map[string]any{
		"block": []*Block{{
			Name:  "a",
			Start: StartSpec{Fixed: []string{"~"}},
			End:   EndSpec{Fixed: []string{")"}, Consume: false},
		}},
	})
	parseEq(t, j1, `(~hi)`, "hi")

	// consume: true removes the end delimiter.
	j2 := makeMini(t, map[string]any{
		"block": []*Block{{
			Name:  "a",
			Start: StartSpec{Fixed: []string{"~"}},
			End:   EndSpec{Fixed: []string{";", ""}, Consume: true},
		}},
	})
	parseEq(t, j2, `~hi;`, "hi")
}

func TestRuleContextParentExclude(t *testing.T) {
	j := makeMini(t, map[string]any{
		"block": []*Block{{
			Name: "at",
			Start: StartSpec{
				Fixed: []string{"@"},
				Rule:  &HooverRuleSpec{Parent: &HooverRuleFilter{Exclude: []string{"group"}}},
			},
			End: EndSpec{Fixed: []string{"@"}},
		}},
	})
	parseEq(t, j, `@hi@`, "hi")     // top level: parent not group -> matches
	parseEq(t, j, `(@hi@)`, "@hi@") // inside group: excluded -> text token
}

func TestRuleContextCurrentFilter(t *testing.T) {
	j := makeMini(t, map[string]any{
		"block": []*Block{{
			Name: "at",
			Start: StartSpec{
				Fixed: []string{"@"},
				Rule: &HooverRuleSpec{
					Current: &HooverRuleFilter{Include: []string{"val"}, Exclude: []string{"group"}},
				},
			},
			End: EndSpec{Fixed: []string{"@"}},
		}},
	})
	parseEq(t, j, `@hi@`, "hi")
}

func TestRuleContextState(t *testing.T) {
	// explicit state "oc" checks open|close; matches at val open.
	j1 := makeMini(t, map[string]any{
		"block": []*Block{{
			Name:  "at",
			Start: StartSpec{Fixed: []string{"@"}, Rule: &HooverRuleSpec{State: "oc"}},
			End:   EndSpec{Fixed: []string{"@"}},
		}},
	})
	parseEq(t, j1, `@hi@`, "hi")

	// state "" + parent filter still matches at val open (default "o").
	j2 := makeMini(t, map[string]any{
		"block": []*Block{{
			Name: "at",
			Start: StartSpec{
				Fixed: []string{"@"},
				Rule:  &HooverRuleSpec{Parent: &HooverRuleFilter{Include: []string{"group"}}, State: ""},
			},
			End: EndSpec{Fixed: []string{"@"}},
		}},
	})
	parseEq(t, j2, `(@hi@)`, "hi")

	// StateAny skips the state check entirely (mirrors the TS
	// `state: ''` case of the "rule context state" test — in TS the
	// empty string means "don't check", which Go expresses with the
	// dedicated StateAny sentinel since "" defaults to "o").
	j3 := makeMini(t, map[string]any{
		"block": []*Block{{
			Name: "at",
			Start: StartSpec{
				Fixed: []string{"@"},
				Rule:  &HooverRuleSpec{Parent: &HooverRuleFilter{Include: []string{"group"}}, State: StateAny},
			},
			End: EndSpec{Fixed: []string{"@"}},
		}},
	})
	parseEq(t, j3, `(@hi@)`, "hi")
}

// TestParseToEnd exercises the exported ParseToEnd wrapper directly,
// mirroring the TS module's exported parseToEnd.
func TestParseToEnd(t *testing.T) {
	// Happy path: scan to the ';' end delimiter and consume it.
	lex := tabnas.NewLex("hello;rest", tabnas.DefaultLexConfig())
	pnt := lex.Cursor()
	hvpnt := &tabnas.Point{Len: pnt.Len, SI: pnt.SI, RI: pnt.RI, CI: pnt.CI}
	block := &Block{Name: "t", End: EndSpec{Fixed: []string{";"}}}

	res := ParseToEnd(lex, hvpnt, block, nil)
	if !res.Done {
		t.Fatalf("ParseToEnd not done: %#v", res)
	}
	if res.Val != "hello" {
		t.Errorf("Val = %#v, want %q", res.Val, "hello")
	}
	if res.Err != "" {
		t.Errorf("Err = %q, want empty", res.Err)
	}
	if hvpnt.SI != len("hello;") {
		t.Errorf("hvpnt.SI = %d, want %d (end delimiter consumed)", hvpnt.SI, len("hello;"))
	}

	// Escape handling: unknown escape with AllowUnknownEscape=false errors.
	lex2 := tabnas.NewLex(`a\zb;`, tabnas.DefaultLexConfig())
	pnt2 := lex2.Cursor()
	hvpnt2 := &tabnas.Point{Len: pnt2.Len, SI: pnt2.SI, RI: pnt2.RI, CI: pnt2.CI}
	allow := false
	block2 := &Block{
		Name:               "t",
		End:                EndSpec{Fixed: []string{";"}},
		EscapeChar:         `\`,
		AllowUnknownEscape: &allow,
	}
	res2 := ParseToEnd(lex2, hvpnt2, block2, nil)
	if res2.Done {
		t.Errorf("expected not done on invalid escape, got %#v", res2)
	}
	if res2.Err != "invalid_escape" {
		t.Errorf("Err = %q, want %q", res2.Err, "invalid_escape")
	}
}

func TestNewlineTerminatedValue(t *testing.T) {
	j := makeMini(t, map[string]any{
		"block": []*Block{{
			Name:  "line",
			Start: StartSpec{Fixed: []string{"~"}},
			End:   EndSpec{Fixed: []string{"\n", "\r\n", ""}},
		}},
	})
	parseEq(t, j, "~a b\n", "a b")   // newline consumed
	parseEq(t, j, "~a b\r\n", "a b") // CRLF consumed
	parseEq(t, j, "~a b", "a b")     // EOF
}

func TestNoPanicOnMalformedOptions(t *testing.T) {
	// hoover must never panic; any failure is a returned error. Exercise a
	// range of malformed/edge option shapes.
	cases := []map[string]any{
		nil,
		{},
		{"lex": "nonsense"},
		{"lex": map[string]any{}},
		{"lex": map[string]any{"order": "notint"}},
		{"lex": map[string]any{"order": 3.5}},
		{"block": "notblocks"},
		{"action": "notanaction"},
	}
	for i, opts := range cases {
		func() {
			defer func() {
				if r := recover(); r != nil {
					t.Errorf("case %d (%v): panicked: %v", i, opts, r)
				}
			}()
			j := tabnas.Make()
			_ = j.Use(miniGrammar)
			// Use (not UseDefaults) so Defaults are not merged in; the plugin
			// must still register or return an error, never panic.
			if err := j.Use(Hoover, opts); err != nil {
				return // an error is acceptable; a panic is not
			}
			// A registered instance must also parse without panicking.
			_, _ = j.Parse(`'''x'''`)
		}()
	}
}

func TestRegistersWithoutLexOrder(t *testing.T) {
	// A direct Use (no Defaults merge) and no lex option must register
	// cleanly using the default order, not panic.
	j := tabnas.Make()
	if err := j.Use(miniGrammar); err != nil {
		t.Fatalf("miniGrammar: %v", err)
	}
	if err := j.Use(Hoover, map[string]any{
		"block": []*Block{{
			Name:  "tq",
			Start: StartSpec{Fixed: []string{"'''"}},
			End:   EndSpec{Fixed: []string{"'''"}},
		}},
	}); err != nil {
		t.Fatalf("hoover Use: %v", err)
	}
	parseEq(t, j, `'''x'''`, "x")
}

func TestDoesNotMutateCallerBlocks(t *testing.T) {
	block := &Block{
		Name:  "tq",
		Start: StartSpec{Fixed: []string{"'''"}},
		End:   EndSpec{Fixed: []string{"'''"}},
	}
	j := tabnas.Make()
	if err := j.Use(miniGrammar); err != nil {
		t.Fatalf("miniGrammar: %v", err)
	}
	if err := j.UseDefaults(Hoover, Defaults, map[string]any{
		"block": []*Block{block},
	}); err != nil {
		t.Fatalf("hoover Use: %v", err)
	}
	// The default token and token id are applied to an internal copy.
	if block.Token != "" {
		t.Errorf("caller block mutated: Token = %q, want empty", block.Token)
	}
	if block.tin != 0 {
		t.Errorf("caller block mutated: tin = %d, want 0", block.tin)
	}
}

func TestResolvesKeywordValues(t *testing.T) {
	// A hoovered value that matches a defined value (true/false/null)
	// resolves to that value, not the string.
	j := makeMini(t, map[string]any{
		"block": []*Block{{
			Name:  "a",
			Start: StartSpec{Fixed: []string{"~"}},
			End:   EndSpec{Fixed: []string{">", ""}},
		}},
	})
	parseEq(t, j, "~true>", true)
	parseEq(t, j, "~null>", nil)
	parseEq(t, j, "~hello>", "hello") // non-keyword stays a string
}

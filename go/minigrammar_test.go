/* Copyright (c) 2021-2026 Richard Rodger and other contributors, MIT License */

package hoover

import (
	tabnas "github.com/tabnas/parser/go"
)

// miniGrammar is a deliberately tiny, bespoke grammar — not JSON — that
// provides just enough structure to exercise the hoover plugin without
// pulling in a full grammar package. hoover's only production dependency
// is the tabnas engine; this grammar lives in test code.
//
// Shape:
//
//	value := scalar | '(' value ')'
//
// A scalar is any built-in value token (text/number/string/keyword) or a
// hoover token. Parentheses group a single value, which gives the inner
// `val` rule a `group` parent — enough to test rule-context (parent)
// matching. hoover registers its block token as an extra `val` alternate,
// so the engine's `val` rule is the integration point.
func miniGrammar(j *tabnas.Tabnas, _ map[string]any) error {
	op, cp := "(", ")"
	j.SetOptions(tabnas.Options{
		Fixed: &tabnas.FixedOptions{
			Token: map[string]*string{"#OP": &op, "#CP": &cp},
		},
		Rule: &tabnas.RuleOptions{Start: "val"},
	})
	OP := j.Token("#OP")
	CP := j.Token("#CP")

	// val: a scalar value, or a parenthesised group.
	j.Rule("val", func(rs *tabnas.RuleSpec, _ *tabnas.Parser) {
		rs.BO = []tabnas.StateAction{func(r *tabnas.Rule, ctx *tabnas.Context) {
			r.Node = tabnas.Undefined
		}}
		rs.BC = []tabnas.StateAction{func(r *tabnas.Rule, ctx *tabnas.Context) {
			if !tabnas.IsUndefined(r.Node) {
				return
			}
			if r.Child != nil && !tabnas.IsUndefined(r.Child.Node) {
				r.Node = r.Child.Node
				return
			}
			if r.OS == 0 {
				return
			}
			r.Node = r.O0.ResolveVal(r, ctx)
		}}
		rs.Open = []*tabnas.AltSpec{
			{S: [][]tabnas.Tin{{OP}}, P: "group", B: 1},
			{S: [][]tabnas.Tin{tabnas.TinSetVAL}},
		}
		rs.Close = []*tabnas.AltSpec{
			{S: [][]tabnas.Tin{{tabnas.TinZZ}}},
			{S: [][]tabnas.Tin{{CP}}, B: 1},
		}
	})

	// group: '(' value ')' — yields the inner value.
	j.Rule("group", func(rs *tabnas.RuleSpec, _ *tabnas.Parser) {
		rs.BC = []tabnas.StateAction{func(r *tabnas.Rule, ctx *tabnas.Context) {
			if r.Child != nil {
				r.Node = r.Child.Node
			}
		}}
		rs.Open = []*tabnas.AltSpec{
			{S: [][]tabnas.Tin{{OP}}, P: "val"},
		}
		rs.Close = []*tabnas.AltSpec{
			{S: [][]tabnas.Tin{{CP}}},
		}
	})
	return nil
}

// makeMini builds a bare engine, installs the mini grammar (the
// dependency), then the hoover plugin with the given blocks/options.
func makeMini(t testingT, opts map[string]any) *tabnas.Tabnas {
	j := tabnas.Make()
	if err := j.Use(miniGrammar); err != nil {
		t.Fatalf("miniGrammar: %v", err)
	}
	if err := j.UseDefaults(Hoover, Defaults, opts); err != nil {
		t.Fatalf("hoover Use: %v", err)
	}
	return j
}

// testingT is the subset of *testing.T used by the helpers.
type testingT interface {
	Fatalf(format string, args ...any)
}

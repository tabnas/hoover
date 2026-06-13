/* Copyright (c) 2021-2026 Richard Rodger and other contributors, MIT License */

package hoover

import (
	"reflect"
	"testing"

	jsonic "github.com/jsonicjs/jsonic/go"
)

func TestTripleQuote(t *testing.T) {
	j := jsonic.Make()
	j.UseDefaults(Hoover, Defaults, map[string]any{
		"block": []*Block{
			{
				Name: "triplequote",
				Start: StartSpec{
					Fixed: []string{"'''"},
				},
				End: EndSpec{
					Fixed: []string{"'''"},
				},
			},
		},
	})

	tests := []struct {
		name string
		src  string
		want any
	}{
		{"object value", `{a:'''x'''}`, map[string]any{"a": "x"}},
		{"array value", `['''x''']`, []any{"x"}},
		{"pair value", `a:'''x'''`, map[string]any{"a": "x"}},
		{"pair array value", `a:['''x''']`, map[string]any{"a": []any{"x"}}},
		{"top level", `'''x'''`, "x"},

		{"object newline", "{a:'''\nx\n'''}", map[string]any{"a": "\nx\n"}},
		{"object newlines spaces", "{a:'''\n\n  x\n\n'''}", map[string]any{"a": "\n\n  x\n\n"}},

		{"array newline", "['''\nx\n''']", []any{"\nx\n"}},
		{"array newlines spaces", "['''\n\n  x\n\n''']", []any{"\n\n  x\n\n"}},

		{"pair newline", "a:'''\nx\n'''", map[string]any{"a": "\nx\n"}},
		{"pair newlines spaces", "a:'''\n\n  x\n\n'''", map[string]any{"a": "\n\n  x\n\n"}},

		{"pair array newline", "a:['''\nx\n''']", map[string]any{"a": []any{"\nx\n"}}},
		{"pair array newlines spaces", "a:['''\n\n  x\n\n''']", map[string]any{"a": []any{"\n\n  x\n\n"}}},

		{"top newline", "'''\nx\n'''", "\nx\n"},
		{"top newlines spaces", "'''\n\n  x\n\n'''", "\n\n  x\n\n"},

		{"mixed content", `{a:1,b:'x',c:['y'] d:e:'z', ` + "\n" + `f:"'''"}`,
			map[string]any{
				"a": float64(1),
				"b": "x",
				"c": []any{"y"},
				"d": map[string]any{"e": "z"},
				"f": "'''",
			}},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			got, err := j.Parse(tt.src)
			if err != nil {
				t.Fatalf("Parse(%q) error: %v", tt.src, err)
			}
			if !reflect.DeepEqual(got, tt.want) {
				t.Errorf("Parse(%q)\n  got  %#v\n  want %#v", tt.src, got, tt.want)
			}
		})
	}
}

func TestEndOfLine(t *testing.T) {
	j := jsonic.Make()
	j.UseDefaults(Hoover, Defaults, map[string]any{
		// Mirror the TS endofline test: run after string and number matchers.
		"lex": map[string]any{"order": 7500000},
		"block": []*Block{
			{
				Name: "endofline",
				Start: StartSpec{
					Rule: &HooverRuleSpec{
						Parent: &HooverRuleFilter{
							Include: []string{"pair", "elem"},
						},
					},
				},
				End: EndSpec{
					Fixed:   []string{"\n", "\r\n", "#", ";", ""},
					Consume: []string{"\n", "\r\n"},
				},
				EscapeChar: "\\",
				Escape: map[string]string{
					"#":  "#",
					";":  ";",
					"\\": "\\",
				},
				Trim: true,
			},
		},
	})

	tests := []struct {
		name string
		src  string
		want any
	}{
		{"basic spaces", "{a:x x\n}", map[string]any{"a": "x x"}},

		{"multiline pair", "\n    a: x x\n    ",
			map[string]any{"a": "x x"}},

		// NOTE: does not lex at top level
		{"hash as end", `x: a#b`, map[string]any{"x": "a"}},
		{"escaped hash", `x:a\#b`, map[string]any{"x": "a#b"}},

		// NOTE: d:e:'z' will no longer work
		{"mixed content", "{ a: 1, b: 'x', c: ['y'], \nf: \"'''\" }",
			map[string]any{
				"a": float64(1),
				"b": "x",
				"c": []any{"y"},
				"f": "'''",
			}},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			got, err := j.Parse(tt.src)
			if err != nil {
				t.Fatalf("Parse(%q) error: %v", tt.src, err)
			}
			if !reflect.DeepEqual(got, tt.want) {
				t.Errorf("Parse(%q)\n  got  %#v\n  want %#v", tt.src, got, tt.want)
			}
		})
	}
}

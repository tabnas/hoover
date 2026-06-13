/* Copyright (c) 2021-2026 Richard Rodger and other contributors, MIT License */

package hoover

import (
	"bufio"
	"encoding/json"
	"fmt"
	"math"
	"os"
	"path/filepath"
	"reflect"
	"strings"
	"testing"

	tabnas "github.com/tabnas/parser/go"
	jsonic "github.com/tabnas/parser/go/jsonic"
)

// tsvRow holds a row from a TSV file.
type tsvRow struct {
	cols   []string
	lineNo int
}

// loadTSV reads a TSV file and returns its rows (excluding the header).
func loadTSV(path string) ([]tsvRow, error) {
	f, err := os.Open(path)
	if err != nil {
		return nil, err
	}
	defer f.Close()

	var rows []tsvRow
	scanner := bufio.NewScanner(f)
	lineNo := 0
	for scanner.Scan() {
		lineNo++
		if lineNo == 1 {
			continue // skip header
		}
		line := scanner.Text()
		if line == "" {
			continue
		}
		cols := strings.Split(line, "\t")
		rows = append(rows, tsvRow{cols: cols, lineNo: lineNo})
	}
	return rows, scanner.Err()
}

// parseExpected parses the expected JSON string into a Go value.
func parseExpected(s string) (any, error) {
	if s == "" {
		return nil, nil
	}
	var val any
	err := json.Unmarshal([]byte(s), &val)
	if err != nil {
		return nil, err
	}
	return val, nil
}

func formatValue(v any) string {
	if v == nil {
		return "nil"
	}
	b, err := json.Marshal(v)
	if err != nil {
		return fmt.Sprintf("%v", v)
	}
	return string(b)
}

func normalizeValue(v any) any {
	switch val := v.(type) {
	case map[string]any:
		result := make(map[string]any)
		for k, v := range val {
			result[k] = normalizeValue(v)
		}
		return result
	case []any:
		result := make([]any, len(val))
		for i, v := range val {
			result[i] = normalizeValue(v)
		}
		return result
	case float64:
		if val == 0 {
			return float64(0)
		}
		return val
	default:
		return v
	}
}

func valuesEqual(got, expected any) bool {
	return deepCompare(normalizeValue(got), normalizeValue(expected))
}

func deepCompare(a, b any) bool {
	if a == nil && b == nil {
		return true
	}
	if a == nil || b == nil {
		return false
	}
	switch av := a.(type) {
	case map[string]any:
		bv, ok := b.(map[string]any)
		if !ok || len(av) != len(bv) {
			return false
		}
		for k, v := range av {
			if !deepCompare(v, bv[k]) {
				return false
			}
		}
		return true
	case []any:
		bv, ok := b.([]any)
		if !ok || len(av) != len(bv) {
			return false
		}
		for i := range av {
			if !deepCompare(av[i], bv[i]) {
				return false
			}
		}
		return true
	case float64:
		bv, ok := b.(float64)
		if !ok {
			return false
		}
		if math.IsNaN(av) && math.IsNaN(bv) {
			return true
		}
		return av == bv
	case string:
		bv, ok := b.(string)
		return ok && av == bv
	case bool:
		bv, ok := b.(bool)
		return ok && av == bv
	default:
		return reflect.DeepEqual(a, b)
	}
}

func specDir() string {
	return filepath.Join("..", "test", "spec")
}

// unescape converts TSV escape sequences to actual characters.
func unescape(s string) string {
	s = strings.ReplaceAll(s, "\\r\\n", "\r\n")
	s = strings.ReplaceAll(s, "\\n", "\n")
	s = strings.ReplaceAll(s, "\\r", "\r")
	s = strings.ReplaceAll(s, "\\t", "\t")
	return s
}

// runTSV runs a standard 2-column TSV (input, expected) with a given jsonic instance.
func runTSV(t *testing.T, file string, j *tabnas.Tabnas) {
	t.Helper()
	path := filepath.Join(specDir(), file)
	rows, err := loadTSV(path)
	if err != nil {
		t.Fatalf("failed to load %s: %v", file, err)
	}

	for _, row := range rows {
		if len(row.cols) < 2 {
			continue
		}
		input := unescape(row.cols[0])
		expectedStr := row.cols[1]

		expected, err := parseExpected(expectedStr)
		if err != nil {
			t.Errorf("line %d: failed to parse expected %q: %v", row.lineNo, expectedStr, err)
			continue
		}

		got, err := j.Parse(input)
		if err != nil {
			t.Errorf("line %d: Parse(%q) error: %v", row.lineNo, row.cols[0], err)
			continue
		}

		if !valuesEqual(got, expected) {
			t.Errorf("line %d: Parse(%q)\n  got:      %s\n  expected: %s",
				row.lineNo, row.cols[0], formatValue(got), formatValue(expected))
		}
	}
}

// --- Hoover configurations matching the TSV spec files ---

// makeTripleQuote creates a jsonic instance with triple-quote block.
func makeTripleQuote() *tabnas.Tabnas {
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
	return j
}

// makeEndOfLine creates a jsonic instance with endofline block.
// order sets the lex matcher priority.
func makeEndOfLine(order int) *tabnas.Tabnas {
	j := jsonic.Make()
	j.UseDefaults(Hoover, Defaults, map[string]any{
		"lex": map[string]any{
			"order": order,
		},
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
	return j
}

// makeEndOfLineNoNumber creates a jsonic instance with endofline block
// and number lexing disabled. Tests the bug where number-like values
// were skipped by hoover.
func makeEndOfLineNoNumber(order int) *tabnas.Tabnas {
	bFalse := false
	j := jsonic.Make(tabnas.Options{
		Number: &tabnas.NumberOptions{Lex: &bFalse},
	})
	j.UseDefaults(Hoover, Defaults, map[string]any{
		"lex": map[string]any{
			"order": order,
		},
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
					Fixed:   []string{"\n", "\r\n", ""},
					Consume: []string{"\n", "\r\n"},
				},
				Trim: true,
			},
		},
	})
	return j
}

// --- TSV Test Functions ---

func TestTSVBlockFixed(t *testing.T) {
	runTSV(t, "block-fixed.tsv", makeTripleQuote())
}

func TestTSVBlockEndOfLine(t *testing.T) {
	runTSV(t, "block-endofline.tsv", makeEndOfLine(7500000))
}

func TestTSVBlockEndOfLineComment(t *testing.T) {
	runTSV(t, "block-endofline-comment.tsv", makeEndOfLine(7500000))
}

func TestTSVBlockEscape(t *testing.T) {
	runTSV(t, "block-escape.tsv", makeEndOfLine(7500000))
}

func TestTSVBlockTrim(t *testing.T) {
	runTSV(t, "block-trim.tsv", makeEndOfLine(7500000))
}

func TestTSVBlockConsume(t *testing.T) {
	runTSV(t, "block-consume.tsv", makeEndOfLine(7500000))
}

func TestTSVBlockEOF(t *testing.T) {
	runTSV(t, "block-eof.tsv", makeEndOfLine(7500000))
}

func TestTSVBlockStateDefault(t *testing.T) {
	// Tests that the default state is "o" (open only), not "don't check".
	// This was bug #2: Go hoover treated "" as "don't check state"
	// instead of defaulting to "o" like TS.
	runTSV(t, "block-state-default.tsv", makeEndOfLine(7500000))
}

func TestTSVBlockNumberValue(t *testing.T) {
	// Tests that number-like values are matched when number lexing is disabled.
	// This was bug #1: Go hoover skipped isNumberStart chars unconditionally.
	runTSV(t, "block-number-value.tsv", makeEndOfLineNoNumber(7500000))
}

func TestTSVBlockOrder(t *testing.T) {
	// Tests that block array order is respected.
	// This was bug #3: Go used map iteration (random order).
	runTSV(t, "block-order.tsv", makeEndOfLine(7500000))
}

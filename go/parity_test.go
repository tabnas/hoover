// Copyright (c) 2025 Richard Rodger and other contributors, MIT License

package tabnashoover

// parity_test.go — cross-runtime conformance, driven by the shared
// `test/spec/*.tsv` fixtures at the repo root (see ../test/AGENTS.md).
//
// The fixture loader, the escape codec, the ERROR: contract and the row
// loop all come from github.com/tabnas/support/go, whose TypeScript half
// ts/test/parity.test.ts uses to run the SAME files — so the two
// implementations cannot drift without one of them going red, and neither
// can the two loaders.
//
// What is left here is only what is specific to hoover: the grammar it
// extends, and what an ERROR: cell means.

import (
	"encoding/json"
	"strconv"
	"strings"
	"testing"

	tabnas "github.com/tabnas/parser/go"
	support "github.com/tabnas/support/go"
)

// TestSpec runs every fixture in the spec directory. FindSpecDir walks up
// from the package directory, and Dir discovers the files by listing, so
// adding a .tsv runs it in both runtimes without touching either runner.
func TestSpec(t *testing.T) {
	dir, err := support.FindSpecDir("")
	if err != nil {
		t.Fatal(err)
	}

	support.Runner{
		// The runner's own decoding of the input column is bypassed — see
		// specUnescape below — so the raw cell is read and decoded here.
		ParseRow: func(_ string, row *support.Row) (any, error) {
			input := specUnescape(row.Named("input"))

			opts := map[string]any{}
			if raw := row.Named("opts"); "" != strings.TrimSpace(raw) {
				if err := json.Unmarshal([]byte(raw), &opts); err != nil {
					return nil, err
				}
			}

			// hoover has no grammar of its own: it extends whatever
			// grammar supplies the `val` rule. The tiny local mini-grammar
			// plays that part in both runtimes.
			j := tabnas.Make()
			if err := j.Use(miniGrammar); err != nil {
				return nil, err
			}
			if err := j.UseDefaults(Hoover, Defaults, opts); err != nil {
				return nil, err
			}
			return j.Parse(input)
		},

		// hoover's ERROR:<want> cells name a POSITION — 1:8, the line and
		// column the rejection is reported at — not an error code. That is
		// the thing worth pinning for a plugin whose job is to consume text
		// up to a delimiter: rejecting at the wrong place is a different
		// defect from rejecting for the wrong reason. So it is matched
		// against the rendered message, and a bare ERROR still accepts any
		// failure.
		MatchError: func(err error, want string, _ *support.Row) bool {
			return strings.Contains(err.Error(), want)
		},

		// Flatten through JSON so the parser's own containers and numeric
		// types compare against the fixture's decoded shape.
		Normalize: jsonFlatten,
	}.Dir(t, dir)
}

// specUnescape is the one thing this repo does not take from the support
// module: its own escape codec, because hoover's fixtures need a sixth
// escape.
//
// \uXXXX names a code point that must not appear literally in the file: a
// NUL would make git treat the .tsv as binary, and a BOM or a non-ASCII
// space is invisible in a diff. The shared codec passes \u through on
// purpose — a fixture has to be able to carry a literal one — so it is
// decoded here, in one pass over the RAW cell; after the shared codec, a
// plain \u0000 and an escaped-backslash \\u0000 are the same characters.
//
// Non-BMP code points are out of scope: TS decodes to one UTF-16 code unit
// and Go to the rune's UTF-8 bytes, which agree on the BMP only, so a
// fixture must not write a lone surrogate.
//
// Kept byte-identical to unescapeHoover in ts/test/parity.test.ts.
func specUnescape(s string) string {
	if !strings.Contains(s, `\`) {
		return s
	}

	var b strings.Builder
	b.Grow(len(s))
	for i := 0; i < len(s); i++ {
		c := s[i]
		if '\\' == c && i+1 < len(s) {
			switch s[i+1] {
			case 'n':
				b.WriteByte('\n')
				i++
				continue
			case 'r':
				b.WriteByte('\r')
				i++
				continue
			case 't':
				b.WriteByte('\t')
				i++
				continue
			case '\\':
				b.WriteByte('\\')
				i++
				continue
			case 'u':
				if i+5 < len(s) {
					if cp, err := strconv.ParseUint(s[i+2:i+6], 16, 32); err == nil {
						b.WriteRune(rune(cp))
						i += 5
						continue
					}
				}
			}
		}
		b.WriteByte(c)
	}
	return b.String()
}

// jsonFlatten renders a value as JSON and reads it back as plain
// map/slice/float64/string/bool/nil. A value that will not marshal is
// returned as it is: the comparison then fails and prints it, which says
// more than a panic here would.
func jsonFlatten(v any) any {
	raw, err := json.Marshal(v)
	if err != nil {
		return v
	}
	var out any
	if err := json.Unmarshal(raw, &out); err != nil {
		return v
	}
	return out
}

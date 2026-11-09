// Copyright (c) 2026 Richard Rodger and other contributors, MIT License

package tabnashoover

// window_test.go — a parse from a reader (Tabnas.ParseReader, admin ADR-30
// and ADR-33).
//
// hoover is a plugin other grammars install beneath matchers of their own,
// so it declares no window: the grammar that builds a parser whole declares
// lex.window once every matcher it installs, hoover's among them, reads the
// source only through the window. hoover's matcher does: a start delimiter
// the window ends inside is read on before it is compared, and a block whose
// scan reaches the window's end fills the window and carries on, the end of
// the window being the end of the source only once the source has nothing
// more to give.
//
// These tests declare the window on the mini grammar's parser, as a host
// would on its own, and hold hoover to it: every row of the shared fixtures
// and every block shape parse the same from a reader, in pieces of 1, 3, 64
// and 4096 bytes, as from their text; and so do long blocks.
//
// Go's window sizes are fixed at 64 KiB (the engine's small-window hook is
// unexported), and before each token the window reaches 64 KiB past the
// cursor, so a block meets the window's edge only where it is longer than
// that. TestBlocksCutAtEveryPlace puts what a block decides on (an end
// delimiter of one character and of three, a tail, an escape, the end of
// the source) just past 64 KiB into the block, and moves the block along
// the source a byte at a time, read in pieces of 64 bytes, so that the
// window's edge falls before, inside and after it.

import (
	"encoding/json"
	"errors"
	"fmt"
	"io"
	"path/filepath"
	"reflect"
	"sort"
	"strings"
	"testing"

	tabnas "github.com/tabnas/parser/go"
	support "github.com/tabnas/support/go"
)

// readerMargin is how far past the cursor the window reaches before each
// token is cut: a block that reads further meets the window's edge.
const readerMargin = 64 << 10

// pieces is a reader that hands its bytes over at most size at a time.
type pieces struct {
	text []byte
	at   int
	size int
}

func (p *pieces) Read(buf []byte) (int, error) {
	if p.at == len(p.text) {
		return 0, io.EOF
	}
	n := min(p.size, len(buf), len(p.text)-p.at)
	copy(buf, p.text[p.at:p.at+n])
	p.at += n
	return n, nil
}

// windowed is the mini grammar with hoover installed, and lex.window
// declared on the parser, as a host declares it on the parser it makes
// whole.
func windowed(t testingT, opts map[string]any) *tabnas.Tabnas {
	j := makeMini(t, opts)
	yes := true
	j.SetOptions(tabnas.Options{Lex: &tabnas.LexOptions{Window: &yes}})
	return j
}

// shapeOptions reads hoover's options from the JSON shape the fixtures
// write.
func shapeOptions(t *testing.T, text string) map[string]any {
	t.Helper()
	opts := map[string]any{}
	if err := json.Unmarshal([]byte(text), &opts); err != nil {
		t.Fatalf("options %s: %v", text, err)
	}
	return opts
}

// outcome is what a parse shows: its value, or its error's code, position,
// row, column and token text. An error's excerpt is left out: a parse from
// a reader shows a long line only as far back as the window reaches
// (tabnas/parser DIVERGENCE.md), and that is the one difference allowed.
type outcome struct {
	value any
	err   string
}

func outcomeOf(v any, err error) outcome {
	if err == nil {
		return outcome{value: v}
	}
	var te *tabnas.TabnasError
	if errors.As(err, &te) {
		return outcome{err: fmt.Sprintf("%s at %d %d:%d %q", te.Code, te.Pos, te.Row, te.Col, te.Src)}
	}
	return outcome{err: "not a TabnasError: " + err.Error()}
}

func (o outcome) String() string {
	if o.err != "" {
		return "error " + o.err
	}
	s := fmt.Sprintf("value %v", o.value)
	if len(s) > 300 {
		s = s[:300] + "..."
	}
	return s
}

func (o outcome) same(p outcome) bool {
	return o.err == p.err && reflect.DeepEqual(o.value, p.value)
}

// sameFromReader parses text whole and from a reader in pieces of each
// size, and reports where the two differ. It returns the number of reader
// parses compared.
func sameFromReader(t *testing.T, j *tabnas.Tabnas, at, text string, sizes []int) int {
	t.Helper()
	whole := outcomeOf(j.Parse(text))
	for _, size := range sizes {
		read := outcomeOf(j.ParseReader(&pieces{text: []byte(text), size: size}))
		if !read.same(whole) {
			show := text
			if len(show) > 80 {
				show = show[:80]
			}
			t.Errorf("%s: %q in pieces of %d:\n  whole:  %s\n  reader: %s", at, show, size, whole, read)
		}
	}
	return len(sizes)
}

// shapes are the block shapes the fixtures hold, and the ones ini
// installs, each with documents that put a delimiter, an escape or the end
// of the block at every distance from the window's edge. Kept in step with
// SHAPES in ts/test/window.test.ts and shapes in rs/tests/reader_test.rs.
var shapes = []struct {
	opts string
	docs []string
}{
	{
		`{"block":[{"name":"tq","start":{"fixed":"'''"},"end":{"fixed":"'''"}}]}`,
		[]string{"'''x'''", "'''a b\nc''d'''", "(''' x ''')", "'''open", "''x"},
	},
	{
		`{"block":[{"name":"line","start":{"fixed":"~"},"end":{"fixed":["\r\n","\n",""],"consume":["\r\n","\n"]}}]}`,
		[]string{"~a b\r\n", "~a b\n", "~a b", "~a\rb\r\n", "~a\r", "~\r\n"},
	},
	{
		`{"block":[{"name":"angle","start":{"fixed":"<"},"end":{"fixed":">"},"escapeChar":"\\","escape":{"n":"\n",">":">","\\":"\\"},"allowUnknownEscape":false}]}`,
		[]string{"<a\\>b\\\\c\\nd>", "<a\\", "<a\\zb>", "<abc", "(<x>)"},
	},
	// A block whose start delimiter begins another block's, tried first,
	// and an end delimiter that holds another one, kept.
	{
		`{"block":[{"name":"two","start":{"fixed":"~~"},"end":{"fixed":"!"}},{"name":"one","start":{"fixed":"~"},"end":{"fixed":"!"}}]}`,
		[]string{"~~a!", "~a!", "~~!", "~~", "~"},
	},
	{
		`{"block":[{"name":"tail","start":{"fixed":"<"},"end":{"fixed":["abc","b"],"consume":false}}]}`,
		[]string{"<xabc", "<xab", "<xb", "<xa"},
	},
	{
		`{"block":[{"name":"eol","start":{"rule":{"state":""}},"end":{"fixed":["\n","\r\n",";",""],"consume":["\n","\r\n"]},"escapeChar":"\\","escape":{";":";","\\":"\\"},"allowUnknownEscape":true,"preserveEscapeChar":true,"trim":true}]}`,
		[]string{"  a b \\; c \\x d  \r\n", "a\\", "a;", "\ufeff trimmed \u00a0", "x\U0001d11ey\r\n"},
	},
}

// TestHooverDeclaresNoWindow: the plugin leaves lex.window to the grammar
// that builds the parser whole.
func TestHooverDeclaresNoWindow(t *testing.T) {
	if makeMini(t, shapeOptions(t, shapes[0].opts)).Config().Window {
		t.Error("the Hoover plugin declares lex.window")
	}
}

// TestFixturesFromAReader: every fixture row parses the same from a reader
// as from its text, on a parser that declares the window.
func TestFixturesFromAReader(t *testing.T) {
	dir, err := support.FindSpecDir("")
	if err != nil {
		t.Fatal(err)
	}
	paths, err := filepath.Glob(filepath.Join(dir, "*.tsv"))
	if err != nil || len(paths) == 0 {
		t.Fatalf("no fixtures in %s: %v", dir, err)
	}
	sort.Strings(paths)
	compared := 0
	for _, path := range paths {
		spec, err := support.LoadSpec(path, nil)
		if err != nil {
			t.Fatal(err)
		}
		for _, row := range spec.Rows {
			opts := map[string]any{}
			if raw := row.Named("opts"); strings.TrimSpace(raw) != "" {
				opts = shapeOptions(t, raw)
			}
			at := fmt.Sprintf("%s:%d", filepath.Base(path), row.Line)
			compared += sameFromReader(t, windowed(t, opts), at, specUnescape(row.Named("input")), []int{1, 3, 64, 4096})
		}
	}
	t.Logf("reader fixtures: %d reader parses (100%%)", compared)
	if compared < 200 {
		t.Errorf("only %d reader parses", compared)
	}
}

// TestShapesFromAReader: every block shape's documents parse the same from
// a reader as from their text.
func TestShapesFromAReader(t *testing.T) {
	compared := 0
	for _, shape := range shapes {
		j := windowed(t, shapeOptions(t, shape.opts))
		for _, doc := range shape.docs {
			compared += sameFromReader(t, j, shape.opts, doc, []int{1, 3, 64, 4096})
		}
	}
	if compared < 100 {
		t.Errorf("only %d reader parses", compared)
	}
}

// TestBlocksCutAtEveryPlace: what a block decides on, just past the
// window's 64 KiB, with the block moved along the source a byte at a time
// and the source read in pieces of 64 bytes, so that the window's edge
// falls before, inside and after it.
func TestBlocksCutAtEveryPlace(t *testing.T) {
	at := readerMargin + 20
	fill := func(c string, n int) string { return strings.Repeat(c, n) }
	cases := []struct {
		shape int
		doc   string
	}{
		{0, "'''" + fill("t", at-3) + "'''"},
		{0, "'''" + fill("t", at-3) + "''"},
		{1, "~" + fill("l", at-1) + "\r\n"},
		{1, "~" + fill("l", at-1) + "\r"},
		{1, "~" + fill("l", at+40)},
		{2, "<" + fill("a", at-1) + "\\>b>"},
		{2, "<" + fill("a", at-1) + "\\zb>"},
		{2, "<" + fill("a", at-1) + "\\"},
		{4, "<" + fill("x", at-1) + "abc"},
		{4, "<" + fill("x", at-1) + "ab"},
		{5, fill("e", at) + "\\;f\r\n"},
	}
	compared := 0
	for n, c := range cases {
		j := windowed(t, shapeOptions(t, shapes[c.shape].opts))
		for shift := 0; shift < 64; shift++ {
			doc := fill("(", shift) + c.doc + fill(")", shift)
			compared += sameFromReader(t, j, fmt.Sprintf("case %d, shifted %d", n, shift), doc, []int{64})
		}
		t.Logf("reader cut at every place: %d of %d cases (%d%%)", n+1, len(cases), (n+1)*100/len(cases))
	}
	if compared < 700 {
		t.Errorf("only %d reader parses", compared)
	}
}

// TestLongBlockFromAReader: a block of about 300 KiB, full of escapes, its
// end far past the window, parses the same from a reader as from its text,
// in pieces of several sizes, and so does the same block left open, which
// the reader rejects where the whole text does.
func TestLongBlockFromAReader(t *testing.T) {
	var b strings.Builder
	for i := 0; i < 20000; i++ {
		fmt.Fprintf(&b, "row %d \\> \u00e9\\n", i)
	}
	body := b.String()
	j := windowed(t, shapeOptions(t, shapes[2].opts))
	text := "(<" + body + ">)"
	open := "(<" + body
	if len(text) < 2*readerMargin {
		t.Fatalf("the block is %d bytes", len(text))
	}
	if _, err := j.Parse(text); err != nil {
		t.Fatalf("the long block does not parse: %v", err)
	}
	if _, err := j.Parse(open); err == nil {
		t.Fatal("the open block parses")
	}
	sameFromReader(t, j, "the long block", text, []int{64, 4096, readerMargin + 3})
	sameFromReader(t, j, "the long block left open", open, []int{64, 4096, readerMargin + 3})
}

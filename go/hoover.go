/* Copyright (c) 2021-2026 Richard Rodger, MIT License */

package tabnashoover

import (
	"fmt"
	"strings"

	tabnas "github.com/tabnas/parser/go"
)

const Version = "0.2.3"

// Block defines a hoover block configuration.
type Block struct {
	Name               string
	Start              StartSpec
	End                EndSpec
	Token              string // Token name, default "#HV"
	EscapeChar         string
	Escape             map[string]string
	AllowUnknownEscape *bool // default true
	PreserveEscapeChar bool
	Trim               bool

	tin tabnas.Tin
}

// EndSpec defines how a block ends.
type EndSpec struct {
	Fixed   []string // End delimiter(s)
	Consume any      // bool or []string; nil = true
}

// parseResult is the result of parsing to a block end.
type parseResult struct {
	done bool
	val  any
	err  string
}

// ParseResult is the result of ParseToEnd, mirroring the TS ParseResult
// type. Where the TS result carries an optional bad Token, the Go result
// carries the lex error code in Err (e.g. "invalid_escape"); Err is ""
// when no error occurred.
type ParseResult struct {
	Done bool   // True if an end delimiter (or EOF, when configured) was reached.
	Val  any    // The hoovered value (string, or a resolved keyword value).
	Err  string // Lex error code; "" when none.
}

// HooverRuleFilter defines include/exclude lists for rule matching.
type HooverRuleFilter struct {
	Include []string
	Exclude []string
}

// StateAny is a dedicated sentinel for HooverRuleSpec.State meaning "do not
// check the rule state at all". It mirrors the TS spec value `state: ”`,
// which the Go zero value "" cannot express because an unset State defaults
// to "o". Use it in a data-shaped Block spec exactly like the TS form:
//
//	Rule: &tabnashoover.HooverRuleSpec{State: tabnashoover.StateAny}
const StateAny = "*"

// HooverRuleSpec defines rule context conditions for matching.
type HooverRuleSpec struct {
	Parent  *HooverRuleFilter
	Current *HooverRuleFilter
	// State selects which rule states match: "o"/"c"/"oc" = check the rule
	// state; "" (unset) defaults to "o"; StateAny ("*") = don't check the
	// rule state at all (TS: `state: ''`).
	State string
}

// StartSpec defines how a block starts.
type StartSpec struct {
	Fixed   []string        // Start delimiter(s)
	Consume any             // bool, *bool or []string; nil = true. A []string consumes only the listed delimiters.
	Rule    *HooverRuleSpec // Rule context matching
}

// startResult is the result of matching a block start.
type startResult struct {
	match bool
	start string
}

func (b *Block) allowUnknown() bool {
	return b.AllowUnknownEscape == nil || *b.AllowUnknownEscape
}

// Defaults contains the default Hoover plugin options, matching TS Hoover.defaults.
// These are deep-merged with user-provided options by tabnas.UseDefaults().
var Defaults = map[string]any{
	"lex": map[string]any{
		"order": 4500000, // before string(5e6), number(7e6)
	},
}

// blocksFromOpts accepts the `block` option in either of the two shapes the
// TS plugin accepts: the Go-native []*Block, or plain data (the shape options
// arrive in from JSON/config, where TS just takes an object literal). Without
// the data shape the Go plugin could only be configured from Go source, which
// the TS plugin does not require.
//
// Data-shape fields are matched case-insensitively against the Block field
// names, `fixed` may be a single string or a list, and `consume` may be a
// bool or a list — all as in TS.
func blocksFromOpts(v any) []*Block {
	switch defs := v.(type) {
	case nil:
		return nil
	case []*Block:
		return defs
	case []any:
		out := make([]*Block, 0, len(defs))
		for _, d := range defs {
			if b, ok := d.(*Block); ok {
				out = append(out, b)
				continue
			}
			if m, ok := tabnas.AsStringMap(d); ok {
				out = append(out, blockFromMap(m))
			}
		}
		return out
	case []map[string]any:
		out := make([]*Block, 0, len(defs))
		for _, m := range defs {
			out = append(out, blockFromMap(m))
		}
		return out
	}
	return nil
}

// optAny looks a key up case-insensitively, so both the TS spelling
// (`escapeChar`) and the Go field name (`EscapeChar`) resolve.
func optAny(m map[string]any, key string) (any, bool) {
	if v, ok := m[key]; ok {
		return v, true
	}
	for k, v := range m {
		if strings.EqualFold(k, key) {
			return v, true
		}
	}
	return nil, false
}

func optString(m map[string]any, key string) string {
	if v, ok := optAny(m, key); ok {
		if s, ok := v.(string); ok {
			return s
		}
	}
	return ""
}

func optBool(m map[string]any, key string) bool {
	if v, ok := optAny(m, key); ok {
		if b, ok := v.(bool); ok {
			return b
		}
	}
	return false
}

// optStrings reads a `fixed`-style field: a single string or a list of them.
func optStrings(m map[string]any, key string) []string {
	v, ok := optAny(m, key)
	if !ok {
		return nil
	}
	return toStrings(v)
}

func toStrings(v any) []string {
	switch x := v.(type) {
	case string:
		return []string{x}
	case []string:
		return x
	case []any:
		out := make([]string, 0, len(x))
		for _, e := range x {
			if s, ok := e.(string); ok {
				out = append(out, s)
			}
		}
		return out
	}
	return nil
}

// optConsume reads a `consume` field: a bool, or the list of delimiters to
// consume. Absent means the default (consume).
func optConsume(m map[string]any) any {
	v, ok := optAny(m, "consume")
	if !ok {
		return nil
	}
	if b, ok := v.(bool); ok {
		return b
	}
	return toStrings(v)
}

func filterFromAny(v any) *HooverRuleFilter {
	m, ok := tabnas.AsStringMap(v)
	if !ok {
		return nil
	}
	return &HooverRuleFilter{
		Include: optStrings(m, "include"),
		Exclude: optStrings(m, "exclude"),
	}
}

func ruleSpecFromAny(v any) *HooverRuleSpec {
	m, ok := tabnas.AsStringMap(v)
	if !ok {
		return nil
	}
	rs := &HooverRuleSpec{}
	if p, ok := optAny(m, "parent"); ok {
		rs.Parent = filterFromAny(p)
	}
	if c, ok := optAny(m, "current"); ok {
		rs.Current = filterFromAny(c)
	}
	if v, ok := optAny(m, "state"); ok {
		// TS spells "don't check the state" as `state: ''`, which the Go zero
		// value cannot express (unset means "o"); StateAny is that. A JSON
		// null is NOT the empty string — it means the key carries no value,
		// so leave State unset and let the "o" default apply.
		if str, isStr := v.(string); isStr {
			if str == "" {
				str = StateAny
			}
			rs.State = str
		}
	}
	return rs
}

func blockFromMap(m map[string]any) *Block {
	b := &Block{
		Name:               optString(m, "name"),
		Token:              optString(m, "token"),
		EscapeChar:         optString(m, "escapeChar"),
		PreserveEscapeChar: optBool(m, "preserveEscapeChar"),
		Trim:               optBool(m, "trim"),
	}
	if v, ok := optAny(m, "allowUnknownEscape"); ok {
		if allow, ok := v.(bool); ok {
			b.AllowUnknownEscape = &allow
		}
	}
	if v, ok := optAny(m, "escape"); ok {
		if em, ok := tabnas.AsStringMap(v); ok {
			b.Escape = make(map[string]string, len(em))
			for k, ev := range em {
				if s, ok := ev.(string); ok {
					b.Escape[k] = s
				}
			}
		}
	}
	if v, ok := optAny(m, "start"); ok {
		if sm, ok := tabnas.AsStringMap(v); ok {
			b.Start = StartSpec{
				Fixed:   optStrings(sm, "fixed"),
				Consume: optConsume(sm),
			}
			if r, ok := optAny(sm, "rule"); ok {
				b.Start.Rule = ruleSpecFromAny(r)
			}
		}
	}
	if v, ok := optAny(m, "end"); ok {
		if em, ok := tabnas.AsStringMap(v); ok {
			b.End = EndSpec{
				Fixed:   optStrings(em, "fixed"),
				Consume: optConsume(em),
			}
		}
	}
	return b
}

func buildBlocks(blockDefs []*Block) []*Block {
	blocks := make([]*Block, len(blockDefs))
	for i, block := range blockDefs {
		// Copy so applying defaults (and stashing the token id) does not
		// mutate the caller's Block, matching the TS plugin which builds
		// fresh block objects.
		nb := *block
		if nb.Token == "" {
			nb.Token = "#HV"
		}
		blocks[i] = &nb
	}
	return blocks
}

// Hoover is the plugin function, matching the TS Hoover plugin.
// Use with tabnas.UseDefaults to apply Defaults automatically:
//
//	j.UseDefaults(tabnashoover.Hoover, tabnashoover.Defaults, map[string]any{
//	    "block": []*tabnashoover.Block{ ... },
//	})
var Hoover tabnas.Plugin = func(j *tabnas.Tabnas, opts map[string]any) (err error) {
	// Never panic out of the plugin: convert any unexpected panic into a
	// returned error, matching the engine's no-panic contract.
	defer func() {
		if r := recover(); r != nil {
			err = fmt.Errorf("hoover: %v", r)
		}
	}()

	// Hoover extends the host grammar's `val` rule. Fail fast with a clear
	// error if a grammar providing it has not been registered first, rather
	// than silently creating an empty `val` rule and failing confusingly later.
	// (An empty instance keeps the rule key but with no alternates, so check
	// for usable open alternates, not just key presence.)
	if val, ok := j.RSM()["val"]; !ok || val == nil || len(val.OpenAlts()) == 0 {
		return fmt.Errorf(
			"hoover: the 'val' rule is missing; register a grammar that defines it before the hoover plugin")
	}

	blockDefs := blocksFromOpts(opts["block"])
	action, _ := opts["action"].(tabnas.AltAction)

	blocks := buildBlocks(blockDefs)
	tokenMap := map[string]tabnas.Tin{}

	// Carry the host grammar's active `rule.include` tags on the added alt,
	// mirroring the canonical TypeScript. A host that narrows itself with
	// `rule.include` (as `@tabnas/json` does with `include: "json"`) keeps
	// only alts carrying one of its tags, and an untagged one is dropped.
	//
	// The two engines differ on WHEN that filter runs: Go applies it once,
	// where `rule.include` is set, so a later plugin's untagged alt survives;
	// the TypeScript engine re-applies it on every `options()` call, which
	// discards the alt hoover adds. So this is currently a no-op here and
	// load-bearing there. Tagging in both keeps the two identical whichever
	// way that divergence is resolved. See ../ts/src/hoover.ts.
	var groups string
	if r := j.Options().Rule; r != nil {
		groups = r.Include
	}

	for _, block := range blocks {
		tin := j.Token(block.Token)
		block.tin = tin

		if _, exists := tokenMap[block.Token]; !exists {
			localTin := tin
			j.Rule("val", func(rs *tabnas.RuleSpec, _ *tabnas.Parser) {
				rs.PrependOpen(&tabnas.AltSpec{
					S: [][]tabnas.Tin{{localTin}},
					A: action,
					G: groups,
				})
			})
		}
		tokenMap[block.Token] = tin
	}

	makeHooverMatcher := func(cfg *tabnas.LexConfig, _opts *tabnas.Options) tabnas.LexMatcher {
		var hooverMatcher tabnas.LexMatcher
		hooverMatcher = func(lex *tabnas.Lex, rule *tabnas.Rule) (tkn *tabnas.Token) {
			// Never panic out of the lexer: convert any unexpected panic into
			// a bad token so Parse returns an error instead of crashing.
			defer func() {
				if r := recover(); r != nil {
					tkn = lex.Bad("invalid_text")
				}
			}()
			for _, block := range blocks {
				pnt := lex.Cursor()

				hvpnt := &tabnas.Point{
					Len: pnt.Len,
					SI:  pnt.SI,
					RI:  pnt.RI,
					CI:  pnt.CI,
				}

				sr := matchStart(lex, hvpnt, block)

				if sr.match {
					result := parseToEnd(lex, hvpnt, block, cfg)

					if result.done {
						src := lex.Src[pnt.SI:hvpnt.SI]
						out := lex.Token(block.Token, block.tin, result.val, src)

						pnt.SI = hvpnt.SI
						pnt.RI = hvpnt.RI
						pnt.CI = hvpnt.CI

						return out
					}

					// Once a start matches, the block is committed: a failure to
					// reach an end delimiter is an error, not a fall-through to
					// the next block. Mirror TS, which returns result.bad or a
					// generic invalid_text bad token.
					if result.err != "" {
						return lex.Bad(result.err)
					}
					return lex.Bad("invalid_text")
				}
			}
			return nil
		}
		return hooverMatcher
	}

	j.SetOptions(tabnas.Options{
		Lex: &tabnas.LexOptions{
			Match: map[string]*tabnas.MatchSpec{
				"hoover": {
					Order: lexOrder(opts),
					Make:  makeHooverMatcher,
				},
			},
		},
	})
	return nil
}

// lexOrder reads the configured matcher order, defaulting to the Defaults
// value when the lex option is absent or malformed. Mirrors the TS
// `options.lex?.order`, so a direct Use (without UseDefaults merging
// Defaults) registers cleanly instead of panicking.
func lexOrder(opts map[string]any) int {
	if lex, ok := opts["lex"].(map[string]any); ok {
		if order, ok := lex["order"].(int); ok {
			return order
		}
	}
	return 4500000
}

func matchStart(
	lex *tabnas.Lex,
	hvpnt *tabnas.Point,
	block *Block,
) startResult {
	src := lex.Src
	rule := lex.Ctx.Rule

	sI := hvpnt.SI
	rI := hvpnt.RI
	cI := hvpnt.CI

	start := block.Start
	rulespec := start.Rule

	// Rule context matching
	var matchRule *bool
	setMatch := func(val bool) {
		if matchRule == nil {
			t := val
			matchRule = &t
		} else {
			t := val && *matchRule
			matchRule = &t
		}
	}

	if rulespec != nil {
		if rulespec.Parent != nil {
			if rule == nil || rule.Parent == nil {
				return startResult{match: false}
			}
			if rulespec.Parent.Include != nil {
				setMatch(containsStr(rulespec.Parent.Include, rule.Parent.Name))
			}
			if rulespec.Parent.Exclude != nil {
				setMatch(!containsStr(rulespec.Parent.Exclude, rule.Parent.Name))
			}
		}

		if rulespec.Current != nil {
			if rule == nil {
				return startResult{match: false}
			}
			if rulespec.Current.Include != nil {
				setMatch(containsStr(rulespec.Current.Include, rule.Name))
			}
			if rulespec.Current.Exclude != nil {
				setMatch(!containsStr(rulespec.Current.Exclude, rule.Name))
			}
		}
	}

	// Rule state check: default "o" (open).
	// Matches TS behavior where absent state field defaults to "o" and
	// `state: ''` (Go: StateAny) skips the state check entirely.
	rulestate := "o"
	if rulespec != nil {
		if rulespec.State == StateAny {
			rulestate = ""
		} else if rulespec.State != "" {
			rulestate = rulespec.State
		}
	}
	if rulestate != "" {
		if rule == nil {
			return startResult{match: false}
		}
		setMatch(containsChar(rulestate, rule.State))
	}

	if matchRule != nil && !*matchRule {
		return startResult{match: false}
	}

	// Fixed delimiter matching
	matchFixed := true
	fixed := start.Fixed

	if fixed != nil {
		matchFixed = false

		for _, f := range fixed {
			if sI+len(f) <= len(src) && src[sI:sI+len(f)] == f {
				matchFixed = true

				if shouldConsumeStart(start.Consume, f) {
					endI := sI + len(f)
					for fsI := sI; fsI < endI; fsI++ {
						sI++
						cI++
						if src[fsI] == '\n' {
							rI++
							cI = 1
						}
					}
				}

				break
			}
		}
	}

	if matchFixed {
		startsrc := src[hvpnt.SI:sI]

		if block.Trim {
			startsrc = trimString(startsrc)
		}

		hvpnt.SI = sI
		hvpnt.RI = rI
		hvpnt.CI = cI

		return startResult{
			match: true,
			start: startsrc,
		}
	}

	return startResult{match: false}
}

// ParseToEnd scans lex.Src from hvpnt for one of block's end delimiters,
// handling escape sequences, and returns the hoovered value, advancing
// hvpnt past the (consumed) end delimiter. It is a thin wrapper over the
// internal parser used by the plugin's lex matcher, exported to mirror the
// TS module's exported parseToEnd. cfg may be nil; when non-nil and value
// lexing is enabled, a hoovered value matching a defined keyword (e.g.
// "true") resolves to that keyword's value.
func ParseToEnd(
	lex *tabnas.Lex,
	hvpnt *tabnas.Point,
	block *Block,
	cfg *tabnas.LexConfig,
) ParseResult {
	r := parseToEnd(lex, hvpnt, block, cfg)
	return ParseResult{Done: r.done, Val: r.val, Err: r.err}
}

func parseToEnd(
	lex *tabnas.Lex,
	hvpnt *tabnas.Point,
	block *Block,
	cfg *tabnas.LexConfig,
) parseResult {
	var valc []byte

	src := lex.Src

	endspec := block.End
	fixed := endspec.Fixed

	endchars := make([]byte, len(fixed))
	endseqs := make([]string, len(fixed))
	for i, end := range fixed {
		if len(end) > 0 {
			endchars[i] = end[0]
			endseqs[i] = end[1:]
		} else {
			// Empty string = EOF marker
			endchars[i] = 0
			endseqs[i] = ""
		}
	}

	escapeChar := byte(0)
	if block.EscapeChar != "" {
		escapeChar = block.EscapeChar[0]
	}

	sI := hvpnt.SI
	rI := hvpnt.RI
	cI := hvpnt.CI

	done := false
	endI := sI

	for sI <= len(src) {
		// EOF check
		if sI == len(src) {
			for i, ec := range endchars {
				if ec == 0 && endseqs[i] == "" {
					endI = sI
					done = true
					break
				}
			}
			break
		}

		c := src[sI]

		// Check for end delimiters
		endCharIndex := -1
		for i, ec := range endchars {
			if ec == c {
				endCharIndex = i
				break
			}
		}

		if endCharIndex >= 0 {
			tail := endseqs[endCharIndex]

			if tail == "" {
				// Single char end delimiter
				endI = sI + 1
				done = true
				break
			}

			if sI+1+len(tail) <= len(src) && src[sI+1:sI+1+len(tail)] == tail {
				endI = sI + 1 + len(tail)
				done = true
				break
			}
		}

		// Handle escape sequences
		if escapeChar != 0 && c == escapeChar && sI+1 < len(src) {
			escaped := src[sI+1]
			nextChar := string(escaped)
			if block.Escape != nil {
				if replacement, ok := block.Escape[nextChar]; ok {
					valc = append(valc, []byte(replacement)...)
					sI += 2
					cI += 2
					if replacement == "\n" {
						rI++
						cI = 1
					}
					continue
				}
			}
			if block.allowUnknown() {
				if block.PreserveEscapeChar {
					valc = append(valc, c)
				}
				valc = append(valc, escaped)
				sI += 2
				cI += 2
				if escaped == '\n' {
					rI++
					cI = 1
				}
				continue
			}
			return parseResult{
				done: false,
				val:  "",
				err:  "invalid_escape",
			}
		}

		valc = append(valc, c)
		sI++
		cI++
		if c == '\n' {
			rI++
			cI = 1
		}
	}

	if done {
		if shouldConsumeEnd(endspec, src, sI, endI) {
			for esI := sI; esI < endI; esI++ {
				sI++
				cI++
				if src[esI] == '\n' {
					rI++
					cI = 1
				}
			}
		}

		hvpnt.SI = sI
		hvpnt.RI = rI
		hvpnt.CI = cI
	}

	val := string(valc)

	if block.Trim {
		val = trimString(val)
	}

	// Resolve defined values (e.g. "true" -> true, "null" -> nil)
	var result any = val
	if cfg != nil && cfg.ValueLex && cfg.ValueDef != nil {
		if defVal, ok := cfg.ValueDef[val]; ok {
			result = defVal
		}
	}

	return parseResult{
		done: done,
		val:  result,
	}
}

// shouldConsumeStart reports whether the matched start delimiter should be
// consumed. Mirrors TS: consume unless consume === false; when an array, only
// consume delimiters listed in it; nil means consume.
func shouldConsumeStart(consume any, matched string) bool {
	switch v := consume.(type) {
	case nil:
		return true
	case bool:
		return v
	case *bool:
		return v == nil || *v
	case []string:
		return containsStr(v, matched)
	}
	return true
}

func shouldConsumeEnd(endspec EndSpec, src string, sI, endI int) bool {
	if endspec.Consume == nil {
		return true
	}

	switch v := endspec.Consume.(type) {
	case bool:
		return v
	case []string:
		endfixed := src[sI:endI]
		for _, s := range v {
			if s == endfixed {
				return true
			}
		}
		return false
	}

	return true
}

func containsStr(list []string, s string) bool {
	for _, item := range list {
		if item == s {
			return true
		}
	}
	return false
}

func containsChar(s string, sub string) bool {
	for _, c := range sub {
		for _, sc := range s {
			if c == sc {
				return true
			}
		}
	}
	return false
}

func trimString(s string) string {
	start := 0
	end := len(s)
	for start < end && (s[start] == ' ' || s[start] == '\t' || s[start] == '\r' || s[start] == '\n') {
		start++
	}
	for end > start && (s[end-1] == ' ' || s[end-1] == '\t' || s[end-1] == '\r' || s[end-1] == '\n') {
		end--
	}
	return s[start:end]
}

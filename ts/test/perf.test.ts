/* Copyright (c) 2021-2026 Richard Rodger and other contributors, MIT License */

import { test, describe } from 'node:test'
import { ok } from 'node:assert'

import { Tabnas } from '@tabnas/parser'
import { Hoover } from '../dist/hoover'
import { miniGrammar } from './minigrammar'

// These tests guard against a performance regression where a caller (or a
// future convenience wrapper) rebuilds the engine + grammar + hoover plugin
// on EVERY parse instead of building one instance and reusing it. Building
// the grammar dominates a parse, so a rebuild-per-call path is many times
// slower than reusing one instance.
//
// Hoover is a PLUGIN, not a module with a public parse(src) convenience: the
// caller owns the instance (new Tabnas().use(miniGrammar).use(Hoover)). There
// is therefore nothing for the module to cache. Instead this test pins the
// representative usage contract: a single construction is amortised over N
// parses, so parsing N times on ONE instance must be far cheaper than
// rebuilding the instance for each of those N parses.
//
// The check is machine-INDEPENDENT: it compares rebuild-per-parse against
// instance reuse on the SAME machine in the SAME run, so a slow CI box cannot
// make it flaky (both sides scale together). There is deliberately NO
// wall-clock budget.

describe('hoover perf', () => {
  test('reusing one instance is far cheaper than rebuilding per parse', () => {
    const src = `'''hello world'''`
    const n = 2000

    const opts = {
      block: [
        { name: 'triplequote', start: { fixed: `'''` }, end: { fixed: `'''` } },
      ],
    }

    // build constructs a fresh hoover-enabled instance — the work a
    // rebuild-per-parse path repeats on every call.
    const build = (): Tabnas => {
      const tn = new Tabnas()
      tn.use(miniGrammar)
      tn.use(Hoover, opts)
      return tn
    }

    // Warm both paths so the comparison is steady-state.
    for (let i = 0; i < 50; i++) build().parse(src)
    const reused = build()
    for (let i = 0; i < 50; i++) reused.parse(src)

    // Rebuild the instance for every parse (the regression shape).
    const r0 = process.hrtime.bigint()
    for (let i = 0; i < n; i++) build().parse(src)
    const rebuild = Number(process.hrtime.bigint() - r0)

    // Parse the same N times reusing one instance (the correct shape).
    const u0 = process.hrtime.bigint()
    for (let i = 0; i < n; i++) reused.parse(src)
    const reuse = Number(process.hrtime.bigint() - u0)

    // Reusing one instance must be much cheaper than rebuilding per parse —
    // building the grammar dominates. If someone makes the hot path rebuild
    // per parse, the two converge and this trips. We assert reuse is at least
    // 4x faster (rebuild > 4*reuse); on this tiny grammar the real gap is far
    // larger, so the slack absorbs scheduling noise without an absolute
    // wall-clock budget.
    const ratio = rebuild / reuse
    ok(
      rebuild > 4 * reuse,
      `instance reuse is not meaningfully faster than rebuilding per parse: ` +
        `${n} rebuild-per-parse took ${(rebuild / 1e6).toFixed(2)}ms vs ` +
        `${(reuse / 1e6).toFixed(2)}ms reusing one instance ` +
        `(ratio ${ratio.toFixed(1)}x, want > 4x). A parse hot path appears to ` +
        `rebuild the engine/grammar each call; build one instance ` +
        `(new Tabnas().use(...)) and reuse it.`,
    )
    // eslint-disable-next-line no-console
    console.log(
      `    rebuild-per-parse=${(rebuild / 1e6).toFixed(2)}ms ` +
        `reuse=${(reuse / 1e6).toFixed(2)}ms ratio=${ratio.toFixed(2)}x`,
    )
  })
})

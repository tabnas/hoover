/* Copyright (c) 2021-2026 Richard Rodger and other contributors, MIT License */

import { Tabnas } from 'tabnas'
import { Hoover } from '../dist/hoover'

// miniGrammar is a deliberately tiny, bespoke grammar — not JSON — that
// provides just enough structure to exercise the hoover plugin without a
// full grammar package. hoover's only dependency is the tabnas engine;
// this grammar lives in test code.
//
//   value := scalar | '(' value ')'
//
// A scalar is any built-in value token or a hoover token. Parentheses
// group a single value, which gives the inner `val` rule a `group`
// parent — enough to test rule-context (parent) matching. hoover
// registers its block token as an extra `val` alternate, so the engine's
// `val` rule is the integration point.
export function miniGrammar(am: Tabnas) {
  am.options({
    fixed: { token: { '#OP': '(', '#CP': ')' } },
    rule: { start: 'val' },
    // Define a few keyword values so value resolution is deterministic.
    value: {
      def: { true: { val: true }, false: { val: false }, null: { val: null } },
    },
  })
  am.token('#OP')
  am.token('#CP')

  // val: a scalar value, or a parenthesised group.
  am.rule('val', (rs: any) => {
    rs.bo((r: any) => {
      r.node = undefined
    })
    rs.bc((r: any, ctx: any) => {
      r.node =
        undefined === r.node
          ? undefined === r.child.node
            ? 0 === r.os
              ? undefined
              : r.o0.resolveVal(r, ctx)
            : r.child.node
          : r.node
    })
    rs.open([
      { s: '#OP', p: 'group', b: 1 },
      { s: '#VAL' },
    ])
    rs.close([{ s: '#ZZ' }, { s: '#CP', b: 1 }])
  })

  // group: '(' value ')' — yields the inner value.
  am.rule('group', (rs: any) => {
    rs.bc((r: any) => {
      r.node = r.child.node
    })
    rs.open([{ s: '#OP', p: 'val' }])
    rs.close([{ s: '#CP' }])
  })
}

// makeMini builds a bare engine, installs the mini grammar (the
// dependency), then the hoover plugin with the given blocks/options.
export function makeMini(opts: any): Tabnas {
  const am = new Tabnas()
  am.use(miniGrammar)
  am.use(Hoover, opts)
  return am
}

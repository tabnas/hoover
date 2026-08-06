/* Copyright (c) 2021-2026 Richard Rodger and other contributors, MIT License */

import { Tabnas } from '@tabnas/parser'
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
export function miniGrammar(tn: Tabnas) {
  tn.options({
    fixed: { token: { '#OP': '(', '#CP': ')' } },
    rule: { start: 'val' },
    // Define a few keyword values so value resolution is deterministic.
    value: {
      def: { true: { val: true }, false: { val: false }, null: { val: null } },
    },
  })
  tn.token('#OP')
  tn.token('#CP')

  // val: a scalar value, or a parenthesised group.
  tn.rule('val', (rs: any) => {
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
  tn.rule('group', (rs: any) => {
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
  const tn = new Tabnas()
  tn.use(miniGrammar)
  tn.use(Hoover, opts)
  return tn
}

// tagGrammar is miniGrammar with every alt carrying a `mini` group tag and
// `rule.include` narrowed to it — the shape a strict grammar plugin uses to
// select a subset of a richer default (`@tabnas/json` does exactly this with
// `rule: { include: 'json' }`).
//
// It exists to pin hoover's behaviour under the engine's alt filter, which
// "applies universally, thus also for subsequent rules": an untagged alt
// added by a plugin is discarded on the next `tn.options()` call — including
// hoover's own matcher registration.
export function tagGrammar(tn: Tabnas) {
  tn.options({
    fixed: { token: { '#OP': '(', '#CP': ')' } },
    rule: { start: 'val', include: 'mini' },
    value: {
      def: { true: { val: true }, false: { val: false }, null: { val: null } },
    },
  })
  tn.token('#OP')
  tn.token('#CP')

  tn.rule('val', (rs: any) => {
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
      { s: '#OP', p: 'group', b: 1, g: 'mini' },
      { s: '#VAL', g: 'mini' },
    ])
    rs.close([{ s: '#ZZ', g: 'mini' }, { s: '#CP', b: 1, g: 'mini' }])
  })

  tn.rule('group', (rs: any) => {
    rs.bc((r: any) => {
      r.node = r.child.node
    })
    rs.open([{ s: '#OP', p: 'val', g: 'mini' }])
    rs.close([{ s: '#CP', g: 'mini' }])
  })
}

// makeTagged is makeMini against tagGrammar.
export function makeTagged(opts: any): Tabnas {
  const tn = new Tabnas()
  tn.use(tagGrammar)
  tn.use(Hoover, opts)
  return tn
}

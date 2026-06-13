/* Copyright (c) 2021-2026 Richard Rodger and other contributors, MIT License */

import { test, describe } from 'node:test'
import { deepEqual, throws } from 'node:assert'

import { Tabnas } from 'tabnas'
import { Hoover } from '../dist/hoover'
import { makeMini, miniGrammar } from './minigrammar'

// These tests run the hoover plugin against the tiny local grammar in
// minigrammar.ts (val + parenthesised group). The grammar exists only to
// give hoover something to plug into; hoover's only dependency is the
// tabnas engine.

describe('hoover', () => {
  test('fixed delimiters', () => {
    const j = makeMini({
      block: [
        { name: 'triplequote', start: { fixed: `'''` }, end: { fixed: `'''` } },
      ],
    })
    deepEqual(j.parse(`'''x'''`), 'x')
    deepEqual(j.parse(`'''hello world'''`), 'hello world') // spaces preserved
    deepEqual(j.parse(`'''a\nb'''`), 'a\nb') // newlines preserved
    deepEqual(j.parse(`'''  spaced  '''`), '  spaced  ') // no trim by default
    deepEqual(j.parse(`('''x''')`), 'x') // nested in a group
    deepEqual(j.parse(`(''' a b ''')`), ' a b ')
  })

  test('EOF and multiple end delimiters', () => {
    const j = makeMini({
      block: [
        {
          name: 'tilde',
          start: { fixed: '~' },
          end: { fixed: ['>', '!', ''] },
        },
      ],
    })
    deepEqual(j.parse(`~hello world`), 'hello world') // EOF terminates
    deepEqual(j.parse(`~a>`), 'a') // first delimiter
    deepEqual(j.parse(`~a!`), 'a') // second delimiter
  })

  test('escapes', () => {
    const j = makeMini({
      block: [
        {
          name: 'angle',
          start: { fixed: '<' },
          end: { fixed: '>' },
          escapeChar: '\\',
          escape: { n: '\n', '>': '>', '\\': '\\' },
        },
      ],
    })
    deepEqual(j.parse(`<a\\>b>`), 'a>b') // escaped end delimiter
    deepEqual(j.parse(`<a\\nb>`), 'a\nb') // mapped escape
    deepEqual(j.parse(`<a\\\\b>`), 'a\\b') // escaped backslash
    deepEqual(j.parse(`<a\\zb>`), 'azb') // unknown escape: backslash dropped
  })

  test('reject unknown escape', () => {
    const j = makeMini({
      block: [
        {
          name: 'angle',
          start: { fixed: '<' },
          end: { fixed: '>' },
          escapeChar: '\\',
          escape: { '>': '>' },
          allowUnknownEscape: false,
        },
      ],
    })
    deepEqual(j.parse(`<a\\>b>`), 'a>b') // known escape still works
    throws(() => j.parse(`<a\\zb>`)) // unknown escape rejected
  })

  test('preserve escape char', () => {
    const j = makeMini({
      block: [
        {
          name: 'angle',
          start: { fixed: '<' },
          end: { fixed: '>' },
          escapeChar: '\\',
          preserveEscapeChar: true,
        },
      ],
    })
    deepEqual(j.parse(`<a\\zb>`), 'a\\zb') // escape char kept in output
  })

  test('trim', () => {
    const j = makeMini({
      block: [
        { name: 'angle', start: { fixed: '<' }, end: { fixed: '>' }, trim: true },
      ],
    })
    deepEqual(j.parse(`<  hello  >`), 'hello')
    deepEqual(j.parse(`< a b >`), 'a b') // internal spaces kept, edges trimmed
  })

  test('selective end consume', () => {
    const j = makeMini({
      block: [
        {
          name: 'tilde',
          start: { fixed: '~' },
          end: { fixed: [';', ''], consume: [';'] },
        },
      ],
    })
    deepEqual(j.parse(`~a b;`), 'a b')
    deepEqual(j.parse(`~a b`), 'a b')
  })

  test('rule context parent', () => {
    const j = makeMini({
      block: [
        {
          name: 'at',
          start: { fixed: '@', rule: { parent: { include: ['group'] } } },
          end: { fixed: '@' },
        },
      ],
    })
    // Inside a group (parent === group): matches.
    deepEqual(j.parse(`(@hello world@)`), 'hello world')
    // At top level (parent is not group): does not match, so the bare '@'
    // is unexpected and the parse fails.
    throws(() => j.parse(`@hello world@`))
  })

  test('custom token name', () => {
    const am = new Tabnas()
    am.use(miniGrammar)
    am.use(Hoover, {
      block: [
        {
          name: 'tq',
          token: '#XX',
          start: { fixed: `'''` },
          end: { fixed: `'''` },
        },
      ],
    })
    deepEqual(am.parse(`'''x'''`), 'x')
  })

  test('fail fast on missing grammar', () => {
    // Registering hoover on a bare engine (no grammar, no `val` rule)
    // throws a clear error instead of failing confusingly later.
    const am = new Tabnas()
    throws(() =>
      am.use(Hoover, {
        block: [
          { name: 'tq', start: { fixed: `'''` }, end: { fixed: `'''` } },
        ],
      }),
    )
  })
})

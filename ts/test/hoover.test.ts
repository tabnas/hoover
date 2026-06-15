/* Copyright (c) 2021-2026 Richard Rodger and other contributors, MIT License */

import { test, describe } from 'node:test'
import { deepEqual, throws } from 'node:assert'

import { Tabnas } from '@tabnas/parser'
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
    deepEqual(j.parse('<a\\\nb>'), 'a\nb') // backslash before a literal newline
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
    const tn = new Tabnas()
    tn.use(miniGrammar)
    tn.use(Hoover, {
      block: [
        {
          name: 'tq',
          token: '#XX',
          start: { fixed: `'''` },
          end: { fixed: `'''` },
        },
      ],
    })
    deepEqual(tn.parse(`'''x'''`), 'x')
  })

  test('fail fast on missing grammar', () => {
    // Registering hoover on a bare engine (no grammar, no `val` rule)
    // throws a clear error instead of failing confusingly later.
    const tn = new Tabnas()
    throws(() =>
      tn.use(Hoover, {
        block: [
          { name: 'tq', start: { fixed: `'''` }, end: { fixed: `'''` } },
        ],
      }),
    )
  })

  test('start delimiter consume', () => {
    // consume: false leaves the start delimiter in the value
    const j1 = makeMini({
      block: [
        { name: 'a', start: { fixed: '<', consume: false }, end: { fixed: '>' } },
      ],
    })
    deepEqual(j1.parse(`<hi>`), '<hi')

    // consume: [...] consumes only the listed start delimiters
    const j2 = makeMini({
      block: [
        {
          name: 'a',
          start: { fixed: ['<', '~'], consume: ['<'] },
          end: { fixed: '>' },
        },
      ],
    })
    deepEqual(j2.parse(`<hi>`), 'hi') // '<' consumed
    deepEqual(j2.parse(`~hi>`), '~hi') // '~' kept
  })

  test('end delimiter consume (bool)', () => {
    // consume: false leaves the end delimiter — here the group's ')'
    const j1 = makeMini({
      block: [
        { name: 'a', start: { fixed: '~' }, end: { fixed: ')', consume: false } },
      ],
    })
    deepEqual(j1.parse(`(~hi)`), 'hi')

    // consume: true removes the end delimiter
    const j2 = makeMini({
      block: [
        {
          name: 'a',
          start: { fixed: '~' },
          end: { fixed: [';', ''], consume: true },
        },
      ],
    })
    deepEqual(j2.parse(`~hi;`), 'hi')
  })

  test('rule context parent exclude', () => {
    const j = makeMini({
      block: [
        {
          name: 'at',
          start: { fixed: '@', rule: { parent: { exclude: ['group'] } } },
          end: { fixed: '@' },
        },
      ],
    })
    deepEqual(j.parse(`@hi@`), 'hi') // top level: parent not group → matches
    deepEqual(j.parse(`(@hi@)`), '@hi@') // inside group: excluded → text token
  })

  test('rule context current filter', () => {
    const j = makeMini({
      block: [
        {
          name: 'at',
          start: {
            fixed: '@',
            rule: { current: { include: ['val'], exclude: ['group'] } },
          },
          end: { fixed: '@' },
        },
      ],
    })
    deepEqual(j.parse(`@hi@`), 'hi')
  })

  test('rule context state', () => {
    // explicit state 'oc' checks open|close; matches at val open
    const j1 = makeMini({
      block: [
        { name: 'at', start: { fixed: '@', rule: { state: 'oc' } }, end: { fixed: '@' } },
      ],
    })
    deepEqual(j1.parse(`@hi@`), 'hi')

    // state '' skips the state check entirely
    const j2 = makeMini({
      block: [
        {
          name: 'at',
          start: { fixed: '@', rule: { parent: { include: ['group'] }, state: '' } },
          end: { fixed: '@' },
        },
      ],
    })
    deepEqual(j2.parse(`(@hi@)`), 'hi')
  })

  test('newline-terminated value', () => {
    const j = makeMini({
      block: [
        { name: 'line', start: { fixed: '~' }, end: { fixed: ['\n', '\r\n', ''] } },
      ],
    })
    deepEqual(j.parse('~a b\n'), 'a b') // newline consumed
    deepEqual(j.parse('~a b\r\n'), 'a b') // CRLF consumed
    deepEqual(j.parse('~a b'), 'a b') // EOF
  })

  test('resolves keyword values', () => {
    // A hoovered value that matches a defined value (true/false/null)
    // resolves to that value, not the string.
    const j = makeMini({
      block: [{ name: 'a', start: { fixed: '~' }, end: { fixed: ['>', ''] } }],
    })
    deepEqual(j.parse('~true>'), true)
    deepEqual(j.parse('~null>'), null)
    deepEqual(j.parse('~hello>'), 'hello') // non-keyword stays a string
  })

  test('registers without an explicit lex order', () => {
    // No lex option given: the default order applies and registration works.
    const j = new Tabnas()
      .use(miniGrammar)
      .use(Hoover, {
        block: [{ name: 'tq', start: { fixed: `'''` }, end: { fixed: `'''` } }],
      })
    deepEqual(j.parse(`'''x'''`), 'x')
  })

  test('does not mutate caller block definitions', () => {
    const block: any = {
      name: 'tq',
      start: { fixed: `'''` },
      end: { fixed: `'''` },
    }
    new Tabnas().use(miniGrammar).use(Hoover, { block: [block] })
    // The default token is applied to an internal copy, not the caller's object.
    deepEqual(block.token, undefined)
    deepEqual(block.TOKEN, undefined)
  })
})

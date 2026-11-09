/* Copyright (c) 2026 Richard Rodger and other contributors, MIT License */

// A parse from a reader (`tn.parseReader`, admin ADR-30 and ADR-33).
//
// hoover is a plugin other grammars install beneath matchers of their own,
// so it declares no window: the grammar that builds a parser whole declares
// `lex.window` once every matcher it installs, hoover's among them, reads
// the source only through the window. hoover's matcher does: a start
// delimiter the window ends inside is read on before it is compared, and a
// block whose scan reaches the window's end fills the window and carries
// on, the end of the window being the end of the source only once the
// source has nothing more to give.
//
// These tests declare the window on the mini grammar's parser, as a host
// would on its own, and hold hoover to it: every row of the shared
// fixtures, and blocks of every shape cut at every place, parse the same
// from a reader, in pieces of 1, 3 and 64 bytes, under windows of a few
// UTF-16 units and the engine's own, as from their text; and blocks longer
// than the engine's window, and a long block read under a window of 256
// units that moves on many times, parse the same too.

import { test, describe } from 'node:test'
import assert from 'node:assert'
import { readdirSync } from 'node:fs'
import { join } from 'node:path'
import { inspect } from 'node:util'

import { Tabnas } from '@tabnas/parser'
import { findSpecDir, loadSpec } from '@tabnas/support'

import { makeMini } from './minigrammar'
import { unescapeHoover } from './unescape'

const SPEC = findSpecDir(__dirname)

type Reader = { read(buf: Uint8Array): number }
type Sizes = { margin: number; block: number; chunk: number }
type Seen = { value?: unknown; error?: string }

// A reader that hands its bytes over at most `size` at a time.
function pieces(bytes: Uint8Array, size: number): Reader {
  let at = 0
  return {
    read(buf: Uint8Array): number {
      const n = Math.min(size, buf.length, bytes.length - at)
      buf.set(bytes.subarray(at, at + n))
      at += n
      return n
    },
  }
}

// The mini grammar with hoover installed, and `lex.window` declared on the
// parser, as a host declares it on the parser it makes whole.
function windowed(opts: any): Tabnas {
  const tn = makeMini(opts)
  tn.options({ lex: { window: true } })
  return tn
}

// What a parse shows: its value, or its error's code, position, row,
// column and token text. An error's excerpt is left out: a parse from a
// reader shows a long line only as far back as the window reaches
// (tabnas/parser DIVERGENCE.md), the one difference allowed.
function seen(parse: () => unknown): Seen {
  try {
    return { value: parse() }
  } catch (e: any) {
    if (null == e || 'function' !== typeof e.toJSON) {
      return { error: 'throw ' + (e && e.name) + ' ' + (e && e.message) }
    }
    const d = e.toJSON()
    return {
      error:
        d.code + ' at ' + d.pos + ' ' + d.row + ':' + d.col + ' ' +
        JSON.stringify(d.token.src),
    }
  }
}

const show = (o: Seen): string =>
  undefined !== o.error ? 'error ' + o.error : 'value ' + inspect(o.value, { depth: 4 })

function same(a: Seen, b: Seen): boolean {
  if (a.error !== b.error) return false
  try {
    assert.deepStrictEqual(a.value, b.value)
    return true
  } catch (e) {
    return false
  }
}

// The pieces a reader hands its bytes over in, and the windows (margin and
// block) it is read under besides the engine's own: a few units, so that
// every block meets the window's edge.
const PIECES = [1, 3, 64]
const WINDOWS = [[1, 1], [3, 2], [64, 64]]

// `text` parsed by `tn` whole, and from a reader in every pieces size,
// under the engine's window and each of WINDOWS: how many reader parses
// were compared. A difference is pushed onto `failures`.
function sameFromReader(tn: Tabnas, at: string, text: string, failures: string[]): number {
  const whole = seen(() => tn.parse(text))
  const bytes = Buffer.from(text, 'utf8')
  const anyTn = tn as any
  let compared = 0
  for (const size of PIECES) {
    const reads: [string, () => unknown][] = [['', () => anyTn.parseReader(pieces(bytes, size))]]
    for (const [margin, block] of WINDOWS) {
      const sizes: Sizes = { margin, block, chunk: size }
      reads.push([
        ', margin ' + margin + ', block ' + block,
        () => anyTn.parseReaderWithSizes(pieces(bytes, size), sizes),
      ])
    }
    for (const [how, parse] of reads) {
      const read = seen(parse)
      compared++
      if (!same(read, whole)) {
        failures.push(
          at + ': ' + JSON.stringify(text.slice(0, 80)) + ' from a reader in pieces of ' +
          size + how + '\n  whole:  ' + show(whole) + '\n  reader: ' + show(read),
        )
      }
    }
  }
  return compared
}

// The block shapes the fixtures hold, and the ones ini installs, each with
// documents that put a delimiter, an escape or the end of the block at
// every distance from a window of a few units. Kept in step with `shapes`
// in rs/tests/reader_test.rs and go/window_test.go.
const SHAPES: [any, string[]][] = [
  [
    { block: [{ name: 'tq', start: { fixed: "'''" }, end: { fixed: "'''" } }] },
    ["'''x'''", "'''a b\nc''d'''", "(''' x ''')", "'''open", "''x"],
  ],
  [
    {
      block: [{
        name: 'line', start: { fixed: '~' },
        end: { fixed: ['\r\n', '\n', ''], consume: ['\r\n', '\n'] },
      }],
    },
    ['~a b\r\n', '~a b\n', '~a b', '~a\rb\r\n', '~a\r', '~\r\n'],
  ],
  [
    {
      block: [{
        name: 'angle', start: { fixed: '<' }, end: { fixed: '>' },
        escapeChar: '\\', escape: { n: '\n', '>': '>', '\\': '\\' },
        allowUnknownEscape: false,
      }],
    },
    ['<a\\>b\\\\c\\nd>', '<a\\', '<a\\zb>', '<abc', '(<x>)'],
  ],
  // A block whose start delimiter begins another block's, tried first,
  // and an end delimiter that holds another one, kept: a window that ends
  // inside either must read on before the block decides, or the shorter
  // delimiter wins.
  [
    {
      block: [
        { name: 'two', start: { fixed: '~~' }, end: { fixed: '!' } },
        { name: 'one', start: { fixed: '~' }, end: { fixed: '!' } },
      ],
    },
    ['~~a!', '~a!', '~~!', '~~', '~'],
  ],
  [
    {
      block: [{
        name: 'tail', start: { fixed: '<' },
        end: { fixed: ['abc', 'b'], consume: false },
      }],
    },
    ['<xabc', '<xab', '<xb', '<xa'],
  ],
  [
    {
      block: [{
        name: 'eol', start: { rule: { state: '' } },
        end: { fixed: ['\n', '\r\n', ';', ''], consume: ['\n', '\r\n'] },
        escapeChar: '\\', escape: { ';': ';', '\\': '\\' },
        allowUnknownEscape: true, preserveEscapeChar: true, trim: true,
      }],
    },
    [
      '  a b \\; c \\x d  \r\n',
      'a\\',
      'a;',
      '﻿ trimmed  ',
      'x\u{1d11e}y\r\n',
    ],
  ],
]

describe('window', () => {
  test('hoover-declares-no-window', () => {
    const tn = makeMini(SHAPES[0][0]) as any
    assert.ok(!tn.internal().config.lex.window)
  })

  test('every-row-parses-the-same-from-a-reader', () => {
    let compared = 0
    const failures: string[] = []
    for (const file of readdirSync(SPEC).filter((f) => f.endsWith('.tsv')).sort()) {
      for (const row of loadSpec(join(SPEC, file)).rows) {
        const opts = row.named('opts')
        compared += sameFromReader(
          windowed('' === opts.trim() ? {} : JSON.parse(opts)),
          file + ':' + row.line,
          unescapeHoover(row.named('input')),
          failures,
        )
      }
    }
    console.log('window fixtures: ' + compared + ' reader parses (100%)')
    assert.ok(600 < compared, '' + compared)
    assert.deepEqual(failures, [])
  })

  test('every-block-meets-the-window-edge', () => {
    let compared = 0
    const failures: string[] = []
    for (const [opts, docs] of SHAPES) {
      const tn = windowed(opts)
      for (const doc of docs) {
        compared += sameFromReader(tn, JSON.stringify(opts), doc, failures)
      }
    }
    assert.ok(300 < compared, '' + compared)
    assert.deepEqual(failures, [])
  })

  test('blocks-longer-than-the-window-parse-the-same-from-a-reader', () => {
    // The engine's window reaches 64 KiB past the cursor before each token:
    // a block longer than that meets its edge under the engine's own sizes.
    const long = (64 << 10) + 2048
    const cases: [any, string[]][] = [
      [SHAPES[0][0], ["'''" + 't'.repeat(long) + "'''", "'''" + 't'.repeat(long)]],
      [
        SHAPES[1][0],
        ['~' + 'l'.repeat(long) + '\r\n', '~' + 'l'.repeat(long), '~' + 'l\r'.repeat(long / 2) + '\r'],
      ],
      [
        SHAPES[2][0],
        ['<' + 'a\\>'.repeat(long / 3) + '>', '<' + 'a'.repeat(long) + '\\', '<' + 'a'.repeat(long)],
      ],
      [SHAPES[5][0], ['x\\; é'.repeat(long / 5) + '\u{1d11e}\r\n']],
    ]
    const failures: string[] = []
    for (const [opts, docs] of cases) {
      const tn = windowed(opts) as any
      docs.forEach((doc, n) => {
        const whole = seen(() => tn.parse(doc))
        for (const size of [4096, (64 << 10) + 3]) {
          const read = seen(() => tn.parseReader(pieces(Buffer.from(doc, 'utf8'), size)))
          if (!same(read, whole)) {
            failures.push(JSON.stringify(opts) + ' document ' + n + ' in pieces of ' + size +
              ': whole ' + show(whole).slice(0, 300) + ', reader ' + show(read).slice(0, 300))
          }
        }
      })
    }
    assert.deepEqual(failures, [])
  })

  test('a-long-block-parses-the-same-from-a-reader', () => {
    // Under the engine's window and under one of 256 units that moves on
    // many times before the block's end; and the same block left open,
    // which the reader rejects where the whole text does.
    const rows: string[] = []
    for (let i = 0; i < 20000; i++) rows.push('row ' + i + ' \\> é\\n')
    const body = rows.join('')
    const tn = windowed(SHAPES[2][0]) as any
    ;['(<' + body + '>)', '(<' + body].forEach((doc, n) => {
      const whole = seen(() => tn.parse(doc))
      assert.equal(undefined === whole.error, 0 === n, show(whole).slice(0, 200))
      const bytes = Buffer.from(doc, 'utf8')
      for (const sizes of [undefined, { margin: 256, block: 256, chunk: 4096 }]) {
        const read = seen(() =>
          undefined === sizes
            ? tn.parseReader(pieces(bytes, 4096))
            : tn.parseReaderWithSizes(pieces(bytes, 4096), sizes),
        )
        assert.ok(same(read, whole), show(read).slice(0, 300))
      }
      console.log('window long block: ' + (n + 1) + ' of 2 (' + (n + 1) * 50 + '%)')
    })
  })
})

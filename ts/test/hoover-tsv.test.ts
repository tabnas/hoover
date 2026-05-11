/* Copyright (c) 2021-2026 Richard Rodger and other contributors, MIT License */

import { test, describe } from 'node:test'
import { deepEqual } from 'node:assert'
import { readFileSync } from 'fs'
import { join } from 'path'

import { Jsonic } from 'jsonic'
import { Hoover, Block } from '../dist/hoover'


function unescape(str: string): string {
  return str.replace(/\\r\\n|\\n|\\r|\\t/g, (m) => {
    if (m === '\\r\\n') return '\r\n'
    if (m === '\\n') return '\n'
    if (m === '\\r') return '\r'
    if (m === '\\t') return '\t'
    return m
  })
}


function loadTSV(name: string): { cols: string[]; row: number }[] {
  const specPath = join(__dirname, '..', '..', 'test', 'spec', name + '.tsv')
  const lines = readFileSync(specPath, 'utf8').split(/\r?\n/).filter(Boolean)
  return lines.slice(1).map((line, i) => {
    const cols = line.split('\t').map(unescape)
    return { cols, row: i + 2 }
  })
}


function runTSV(name: string, j: ReturnType<typeof Jsonic.make>) {
  const entries = loadTSV(name)
  for (const { cols: [input, expected], row } of entries) {
    try {
      deepEqual(j(input), JSON.parse(expected))
    } catch (err: any) {
      err.message = `${name}.tsv row ${row}: input=${JSON.stringify(input)} expected=${expected}\n${err.message}`
      throw err
    }
  }
}


// --- Hoover configurations ---

function makeTripleQuote() {
  return Jsonic.make().use(Hoover, {
    block: [
      {
        name: 'triplequote',
        start: { fixed: `'''` },
        end: { fixed: `'''` },
      },
    ],
  })
}

function makeEndOfLine(order: number) {
  return Jsonic.make().use(Hoover, {
    lex: { order },
    block: [
      {
        name: 'endofline',
        start: {
          rule: {
            parent: { include: ['pair', 'elem'] },
          },
        },
        end: {
          fixed: ['\n', '\r\n', '#', ';', ''],
          consume: ['\n', '\r\n'],
        },
        escapeChar: '\\',
        escape: { '#': '#', ';': ';', '\\': '\\' },
        trim: true,
      },
    ],
  })
}

function makeEndOfLineNoNumber(order: number) {
  const j = Jsonic.make().use(Hoover, {
    lex: { order },
    block: [
      {
        name: 'endofline',
        start: {
          rule: {
            parent: { include: ['pair', 'elem'] },
          },
        },
        end: {
          fixed: ['\n', '\r\n', ''],
          consume: ['\n', '\r\n'],
        },
        trim: true,
      },
    ],
  })
  j.options({ number: { lex: false } })
  return j
}


// --- TSV Tests ---

describe('hoover-tsv', () => {

  test('block-fixed', () => {
    runTSV('block-fixed', makeTripleQuote())
  })

  test('block-endofline', () => {
    runTSV('block-endofline', makeEndOfLine(7.5e6))
  })

  test('block-endofline-comment', () => {
    runTSV('block-endofline-comment', makeEndOfLine(7.5e6))
  })

  test('block-escape', () => {
    runTSV('block-escape', makeEndOfLine(7.5e6))
  })

  test('block-trim', () => {
    runTSV('block-trim', makeEndOfLine(7.5e6))
  })

  test('block-consume', () => {
    runTSV('block-consume', makeEndOfLine(7.5e6))
  })

  test('block-eof', () => {
    runTSV('block-eof', makeEndOfLine(7.5e6))
  })

  test('block-state-default', () => {
    // Default state is "o" (open only), not "don't check".
    runTSV('block-state-default', makeEndOfLine(7.5e6))
  })

  test('block-number-value', () => {
    // Number-like values matched when number lexing is disabled.
    runTSV('block-number-value', makeEndOfLineNoNumber(7.5e6))
  })

  test('block-order', () => {
    // Block array order determines iteration priority.
    runTSV('block-order', makeEndOfLine(7.5e6))
  })

})

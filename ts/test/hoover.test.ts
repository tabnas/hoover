/* Copyright (c) 2021 Richard Rodger and other contributors, MIT License */

import { test, describe } from 'node:test'
import { deepEqual } from 'node:assert'

import { Jsonic } from 'jsonic'
import { Hoover } from '../dist/hoover'




describe('hoover', () => {

  test('triplequote', () => {
    const j = Jsonic.make().use(Hoover, {
      block: [
        {
          name: 'triplequote',
          start: {
            fixed: `'''`
          },
          end: {
            fixed: `'''`
          },
        }
      ]
    })

    deepEqual(j(`{a:'''x'''}`), { a: 'x' })
    deepEqual(j(`['''x''']`), ['x'])
    deepEqual(j(`a:'''x'''`), { a: 'x' })
    deepEqual(j(`a:['''x''']`), { a: ['x'] })
    deepEqual(j(`'''x'''`), 'x')

    deepEqual(j(`{a:'''\nx\n'''}`), { a: '\nx\n' })
    deepEqual(j(`{a:'''\n\n  x\n\n'''}`), { a: '\n\n  x\n\n' })

    deepEqual(j(`['''\nx\n''']`), ['\nx\n'])
    deepEqual(j(`['''\n\n  x\n\n''']`), ['\n\n  x\n\n'])

    deepEqual(j(`a:'''\nx\n'''`), { a: '\nx\n' })
    deepEqual(j(`a:'''\n\n  x\n\n'''`), { a: '\n\n  x\n\n' })

    deepEqual(j(`a:['''\nx\n''']`), { a: ['\nx\n'] })
    deepEqual(j(`a:['''\n\n  x\n\n''']`), { a: ['\n\n  x\n\n'] })

    deepEqual(j(`'''\nx\n'''`), '\nx\n')
    deepEqual(j(`'''\n\n  x\n\n'''`), '\n\n  x\n\n')

    deepEqual(j(`{a:1,b:'x',c:['y'] d:e:'z', \nf:"'''"}`),
      { a: 1, b: 'x', c: ['y'], d: { e: 'z' }, f: "'''" })
  })


  test('endofline', () => {
    const j = Jsonic.make()
      .use(Hoover, {
        lex: {
          order: 7.5e6 // before text, after string, number
        },
        block: [
          {
            name: 'endofline',
            start: {
              rule: {
                parent: {
                  include: ['pair', 'elem']
                },
              }
            },
            end: {
              fixed: ['\n', '\r\n', '#', ';', ''],
              consume: ['\n', '\r\n'],
            },
            escapeChar: '\\',
            escape: {
              '#': '#',
              ';': ';',
              '\\': '\\',
            },
            trim: true,
          }
        ]
      })

    deepEqual(j(`{a:x x\n}`), { a: 'x x' })

    deepEqual(j(`
    a: x x
    `),
      {
        a: 'x x'
      })

    // NOTE: does not lex at top level
    deepEqual(j(`x: a#b`), { x: 'a' })
    deepEqual(j(`x:a\\#b`), { x: 'a#b' })

    // NOTE: d:e:'z' will no longer work
    deepEqual(j(`{ a: 1, b: 'x', c: ['y'], \nf: "'''" }`),
      { a: 1, b: 'x', c: ['y'], f: "'''" })
  })

})

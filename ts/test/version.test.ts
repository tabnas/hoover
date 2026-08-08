/* Copyright (c) 2026 Richard Rodger, MIT License */

// The exported VERSION must equal package.json "version".
//
// This is the CI check for version drift. It exists because the constant HAS
// drifted: @tabnas/json exported Version = '1.0.0' for several releases while
// the package shipped 0.4.x, because nothing rewrote it and AGENTS.md wrongly
// claimed `make publish-go` kept it in sync. A release that bumps
// package.json and forgets the constant now fails here.
//
// The requires are deliberate, and deliberately at module scope: if
// package.json cannot be read this file throws and the test run fails, rather
// than skipping a check whose whole point is that it always runs.

import { test, describe } from 'node:test'
import { equal, match } from 'node:assert'

// This file is compiled to dist-test/, so '..' is the ts/ package root:
// '../package.json' is the manifest, and '..' resolves via "main" to the
// built dist/hoover.js — the same entry a consumer's require('@tabnas/hoover')
// gets.
const pkg = require('../package.json')
const api = require('..')

describe('version', () => {
  test('VERSION matches package.json', () => {
    equal(
      api.VERSION,
      pkg.version,
      `VERSION drift: ${pkg.name} exports ${api.VERSION} but package.json is ` +
        `${pkg.version}. Both are rewritten by admin/publish.sh at release; ` +
        `if you bumped one by hand, bump the other.`,
    )
  })

  test('VERSION is exported and looks like a semver', () => {
    equal(typeof api.VERSION, 'string', 'VERSION must be exported as a string')
    match(api.VERSION, /^\d+\.\d+\.\d+/, 'VERSION must be a semver')
  })
})

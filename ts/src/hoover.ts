/* Copyright (c) 2021-2026 Richard Rodger, MIT License */


import {
  Tabnas,
  Plugin,
  Config,
  Lex,
  Point,
  MakeLexMatcher,
  makePoint,
  Token,
  AltAction,
} from '@tabnas/parser'


type Block = {
  name: string
  start?: {
    fixed?: string | string[]
    consume?: null | boolean | string[] // explicit false to turn off
    rule?: {
      parent?: {
        include?: string[]
        exclude?: string[]
      }
      current?: {
        include?: string[]
        exclude?: string[]
      }
      state?: string // '' = don't check, 'o'|'c'|'oc' = check; default 'o'
    }
  }
  end?: {
    fixed: string | string[]
    consume?: null | boolean | string[] // explicit false to turn off
  }
  token?: string
  escapeChar?: string
  escape?: {
    [char: string]: string
  }
  allowUnknownEscape?: boolean
  preserveEscapeChar?: boolean
  trim?: boolean
}


type HooverOptions = {
  block: Block[]
  lex?: {
    order?: number
  }
  action?: AltAction
}


type ParseResult = {
  done: boolean
  val: string
  bad?: Token
}


type StartResult = {
  match: boolean
  start?: string
}


function buildBlocks(blockDefs: Block[]): any[] {
  return blockDefs.map((block) => {
    const out: any = {
      token: '#HV',
      ...block,
    }
    if (null == block.allowUnknownEscape) {
      out.allowUnknownEscape = true
    }
    if (null == block.preserveEscapeChar) {
      out.preserveEscapeChar = false
    }
    return out
  })
}


const Hoover: Plugin = (tn: Tabnas, options: HooverOptions) => {
  // Hoover extends the host grammar's `val` rule. Fail fast with a clear
  // message if a grammar providing it has not been registered first, rather
  // than silently creating an empty `val` rule and failing confusingly later.
  const rules: any = tn.rule()
  if (null == rules || null == rules.val) {
    throw new Error(
      "@tabnas/hoover: the 'val' rule is missing; " +
        'register a grammar that defines it before the hoover plugin',
    )
  }

  let blocks = buildBlocks(options.block)

  // The engine's `rule.include` filter keeps only those alts carrying one of
  // its group tags, and it "applies universally, thus also for subsequent
  // rules" — it is re-applied on every `tn.options()` call, including
  // hoover's own matcher registration below. So on a host that sets it
  // (`@tabnas/json` uses `rule: { include: 'json' }` to narrow the grammar to
  // plain JSON) an untagged alt is added and then silently discarded: the
  // plugin loads, the matcher fires, the `#HV` token is produced — and the
  // parse dies with `unexpected character(s)` pointing at the user's source.
  //
  // Carrying the host's active tags says "this alt is part of the selected
  // grammar", which is exactly what a deliberate `use(Hoover)` means.
  const include: string[] = tn.config().rule.include || []
  const groups = 0 < include.length ? { g: include.slice() } : {}

  let tokenMap: any = {}

  for (let block of blocks) {
    // Create a hoover token
    block.TOKEN = tn.token(block.token)

    if (!tokenMap[block.token]) {
      tn.rule('val', (rs) => {
        rs.open({
          s: [block.TOKEN],
          a: options.action,
          ...groups,
        })
      })
    }

    tokenMap[block.token] = block.TOKEN
  }

  let makeHooverMatcher: MakeLexMatcher = (cfg: Config, _opts) => {
    return function hooverMatcher(lex: Lex) {
      for (let block of blocks) {
        // TODO: Point.clone ?
        const hvpnt = makePoint(lex.pnt.len, lex.pnt.sI, lex.pnt.rI, lex.pnt.cI)

        let startResult = matchStart(lex, hvpnt, block)

        if (startResult.match) {
          let result = parseToEnd(lex, hvpnt, block, cfg)

          if (result.done) {
            let tkn = lex.token(
              block.TOKEN,
              result.val,
              lex.src.substring(lex.pnt.sI, hvpnt.sI),
              hvpnt,
            )
            tkn.use = { block: block.name }

            lex.pnt.sI = hvpnt.sI
            lex.pnt.rI = hvpnt.rI
            lex.pnt.cI = hvpnt.cI

            return tkn
          } else {
            return result.bad || lex.bad('invalid_text', lex.pnt.sI, hvpnt.sI)
          }
        }
      }

      return undefined
    }
  }

  tn.options({
    lex: {
      match: {
        hoover: { order: options.lex?.order, make: makeHooverMatcher },
      },
    },
  })

  // The `tn.options()` call above re-runs the engine's alt filter, so this is
  // the first point at which the alts are known to have survived it. Check
  // rather than assume: the failure mode is silent — a working lexer feeding
  // a rule that cannot accept its token — and it surfaces to the user as an
  // `unexpected character(s)` error on their own source, which points
  // nowhere near the cause.
  const survivors: any[] = (tn.rule() as any).val?.def?.open || []
  const missing = Object.values(tokenMap).filter(
    (tin) =>
      !survivors.some(
        (alt: any) =>
          Array.isArray(alt.s) &&
          alt.s.some((pos: any) =>
            Array.isArray(pos) ? pos.includes(tin) : pos === tin,
          ),
      ),
  )
  if (0 < missing.length) {
    throw new Error(
      '@tabnas/hoover: the block alternate was removed from the ' +
        "'val' rule by the host grammar's alt filter " +
        '(rule.include=' +
        JSON.stringify(include) +
        ', rule.exclude=' +
        JSON.stringify(tn.config().rule.exclude || []) +
        '); hoover cannot extend a grammar that excludes it',
    )
  }
}


function matchStart(
  lex: Lex,
  hvpnt: Point,
  block: Block,
): StartResult {
  let src = lex.src
  let rule = lex.ctx.rule

  let sI = hvpnt.sI // Current point in src
  let rI = hvpnt.rI // Current row
  let cI = hvpnt.cI // Current column

  let start = block.start
  let rulespec = null != start ? start.rule : null

  // `null` means "no rule condition has been evaluated", which is not the
  // same as "a condition failed": with no conditions there is nothing to
  // constrain the match, so it passes. Each check below ANDs itself in.
  let matchRule: null | boolean = null

  // NOTE: Default rule state is open ('o'); there is no default parent or
  // current filter — an absent filter simply imposes no constraint.

  if (null != rulespec) {
    if (rulespec.parent) {
      if (rulespec.parent.include) {
        matchRule =
          rulespec.parent.include.includes(rule.parent.name) &&
          (null === matchRule ? true : matchRule)
      }
      if (rulespec.parent.exclude) {
        matchRule =
          !rulespec.parent.exclude.includes(rule.parent.name) &&
          (null === matchRule ? true : matchRule)
      }
    }

    if (rulespec.current) {
      if (rulespec.current.include) {
        matchRule =
          rulespec.current.include.includes(rule.name) &&
          (null === matchRule ? true : matchRule)
      }
      if (rulespec.current.exclude) {
        matchRule =
          !rulespec.current.exclude.includes(rule.name) &&
          (null === matchRule ? true : matchRule)
      }
    }
  }

  // '': don't check, 'oc'|'c'|'o' check, default 'o'
  let rulestate = null != rulespec && '' === rulespec.state ? '' :
    (null != rulespec ? rulespec.state || 'o' : 'o')
  if (rulestate) {
    matchRule =
      rulestate.includes(rule.state) &&
      (null === matchRule ? true : matchRule)
  }

  // Resolve "no condition evaluated" (null) to a pass. Without this, a
  // rulespec that only turns the state check off (`state: ''`) and sets no
  // parent/current filter would leave matchRule null and be read as a
  // failure, so the block could never match at all — the opposite of what
  // "skip the state check" means. Mirrors the Go port's `matchRule != nil
  // && !*matchRule` early return.
  const ruleMatched: boolean = null === matchRule ? true : matchRule

  let matchFixed = true
  let fixed = null != start ? start.fixed : null
  if (ruleMatched && null != fixed) {
    matchFixed = false

    fixed = Array.isArray(fixed) ? fixed : [fixed]
    for (let fI = 0; !matchFixed && fI < fixed.length; fI++) {
      if (src.substring(hvpnt.sI).startsWith(fixed[fI])) {
        matchFixed = true

        if (false !== start!.consume) {
          if (
            !Array.isArray(start!.consume) ||
            start!.consume.includes(fixed[fI])
          ) {
            let endI = hvpnt.sI + fixed[fI].length
            for (let fsI = hvpnt.sI; fsI < endI; fsI++) {
              sI++
              cI++
              if ('\n' === src[fsI]) {
                rI++
                cI = 1
              }
            }
          }
        }

        break
      }
    }
  }

  if (ruleMatched && matchFixed) {
    let startsrc = src.substring(hvpnt.sI, sI)

    if (false !== block.trim) {
      startsrc = startsrc.trim()
    }

    hvpnt.sI = sI
    hvpnt.rI = rI
    hvpnt.cI = cI

    return {
      match: true,
      start: startsrc,
    }
  } else {
    return { match: false }
  }
}


function parseToEnd(
  lex: Lex,
  hvpnt: Point,
  block: Block,
  cfg: Config,
): ParseResult {
  let valc = []

  let src = lex.src

  let endspec = block.end!
  let fixed: string[] = endspec.fixed as string[]
  fixed = 'string' === typeof fixed ? [fixed] : fixed

  let endchars = fixed.map((end) => end[0])
  let endseqs = fixed.map((end) => end.substring(1))

  let escapeChar = block.escapeChar

  let sI = hvpnt.sI // Current point in src
  let rI = hvpnt.rI // Current row
  let cI = hvpnt.cI // Current column

  let done = false
  let c: string = ''
  let endI = sI
  let endCharIndex = 0

  top: do {
    c = src[sI]

    // Did this step consume a newline? Tracked separately because `c` is
    // not always the source character: an escape may replace it, and with
    // preserveEscapeChar it becomes a two-character sequence that never
    // equals '\n'. Row/column tracking must follow the SOURCE.
    let nl = false

    // Check for end
    if (-1 < (endCharIndex = endchars.indexOf(c))) {
      let tail = endseqs[endCharIndex]

      // EOF
      if (undefined === tail || '' === tail) {
        endI = sI + 1
        done = true
        break top
      }

      // Match tail
      else if (
        'string' === typeof tail &&
        tail === src.substring(sI + 1, sI + 1 + tail.length)
      ) {
        endI = sI + 1 + tail.length
        done = true
        break top
      }
    }

    if (escapeChar === c) {
      // An escape char as the final source character has nothing to escape.
      // It consumes itself and the absent next character, which runs the
      // scan past the end of source, so the block never reaches an end
      // delimiter and the caller reports it as unterminated (invalid_text).
      // Handled explicitly so an `undefined` is never pushed into the value.
      if (src.length <= sI + 1) {
        sI += 2
        break top
      }

      let replacement = block.escape ? block.escape[src[sI + 1]] : undefined

      if (null != replacement) {
        c = replacement
        sI++
        cI++
        nl = '\n' === replacement
      } else if (block.allowUnknownEscape) {
        const escaped = src[sI + 1]
        c = block.preserveEscapeChar ? src.substring(sI, sI + 2) : escaped
        sI++
        // Both source characters (escape char + escaped char) are consumed,
        // so both count towards the column, as in the mapped-escape branch
        // above and in the Go port.
        cI++
        nl = '\n' === escaped
      } else {
        return {
          done: false,
          val: '',
          bad: lex.bad('invalid_escape', sI, sI + 1),
        }
      }
    } else {
      nl = '\n' === c
    }

    valc.push(c)
    sI++
    cI++
    if (nl) {
      rI++
      cI = 1
    }
  } while (sI <= src.length)

  if (done) {
    if (false !== endspec.consume) {
      let endfixed = src.substring(sI, endI)
      if (
        !Array.isArray(endspec.consume) ||
        endspec.consume.includes(endfixed)
      ) {
        let esI = sI
        for (; esI < endI; esI++) {
          sI++
          cI++
          if ('\n' === src[esI]) {
            rI++
            cI = 1
          }
        }
      }
    }

    hvpnt.sI = sI
    hvpnt.rI = rI
    hvpnt.cI = cI
  }

  let val: any = valc.join('')

  if (block.trim) {
    val = val.trim()
  }

  if (cfg.value.lex && undefined !== cfg.value.def[val]) {
    val = cfg.value.def[val].val
  }

  return {
    done,
    val,
  }
}

Hoover.defaults = {
  block: [],
  lex: {
    order: 4.5e6, // before string, number
  },
} as HooverOptions

export { VERSION, parseToEnd, Hoover }

export type { Block, HooverOptions, ParseResult, StartResult }

// VERSION is this package's version. It MUST equal package.json "version":
// the release orchestrator rewrites both, and the version test fails the
// build if they drift. Mirrors `const VERSION` in go/hoover.go.
const VERSION = '0.3.6'

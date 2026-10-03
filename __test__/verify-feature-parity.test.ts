import { test, expect, describe } from 'vitest'
import { execFileSync } from 'node:child_process'
import { cpSync, mkdtempSync, mkdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join, resolve } from 'node:path'

// Proves scripts/verify-feature-parity.cjs fails when the invariant it guards is
// broken. The interesting cases are the ones where a naive implementation passes
// silently: a source that disappears, a step whose indentation changes, two builds
// sharing one step. Each of those was found by mutating a scratch copy of
// `.github/` and observing the exit code, so each is pinned here.
//
// Every helper asserts the mutation actually applied before running the checker.
// A probe whose edit silently no-ops would otherwise report a passing check that
// proves nothing.

const ROOT = resolve(__dirname, '..')

// The checker resolves its own root from __dirname, so each probe must run the
// SANDBOX's copy — running the real one would read the real repo and silently
// ignore every mutation.
const sandboxChecker = (dir: string) => join(dir, 'scripts', 'verify-feature-parity.cjs')

type Mutate = (dir: string) => boolean

/**
 * Copy the files the checker reads into a scratch tree, normalising line endings
 * to LF.
 *
 * A Windows checkout has CRLF, so probes written with `\n` in their search strings
 * would silently fail to match there — and the "mutation applied" assertion below
 * would report a probe bug rather than a checker bug. Normalising here makes every
 * probe see the same bytes on every platform.
 */
function sandbox(): string {
  const dir = mkdtempSync(join(tmpdir(), 'parity-'))
  cpSync(join(ROOT, '.github'), join(dir, '.github'), { recursive: true })
  cpSync(join(ROOT, 'scripts'), join(dir, 'scripts'), { recursive: true })
  cpSync(join(ROOT, 'package.json'), join(dir, 'package.json'))
  for (const rel of [CI, REL, 'package.json']) {
    const p = join(dir, rel)
    writeFileSync(p, readFileSync(p, 'utf8').replaceAll('\r\n', '\n'))
  }
  return dir
}

/** Replace `old` with `new` in a scratch file. Returns false if not found. */
function replace(dir: string, rel: string, old: string, next: string, all = false): boolean {
  const p = join(dir, rel)
  const src = readFileSync(p, 'utf8')
  if (!src.includes(old)) return false
  writeFileSync(p, all ? src.replaceAll(old, next) : src.replace(old, next))
  return true
}

function runChecker(dir: string): { code: number; out: string } {
  try {
    const out = execFileSync(process.execPath, [sandboxChecker(dir)], {
      cwd: dir,
      encoding: 'utf8',
      stdio: ['ignore', 'pipe', 'pipe'],
    })
    return { code: 0, out }
  } catch (e: any) {
    return { code: e.status ?? 1, out: (e.stdout ?? '') + (e.stderr ?? '') }
  }
}

/** Assert the mutation applied, then assert the checker rejects the result. */
function expectRejected(mutate: Mutate, mustMention?: string) {
  const dir = sandbox()
  try {
    expect(mutate(dir), 'probe mutation did not apply — fix the probe').toBe(true)
    const { code, out } = runChecker(dir)
    expect(code, `checker accepted a broken tree:\n${out}`).not.toBe(0)
    if (mustMention) expect(out).toContain(mustMention)
  } finally {
    rmSync(dir, { recursive: true, force: true })
  }
}

function expectAccepted(mutate: Mutate) {
  const dir = sandbox()
  try {
    expect(mutate(dir), 'probe mutation did not apply — fix the probe').toBe(true)
    const { code, out } = runChecker(dir)
    expect(code, `checker rejected a legitimate tree:\n${out}`).toBe(0)
  } finally {
    rmSync(dir, { recursive: true, force: true })
  }
}

const CI = '.github/workflows/ci.yml'
const REL = '.github/workflows/release.yml'

const indentOf = (line: string): number => (line.match(/^[ \t]*/) as RegExpMatchArray)[0].length

/**
 * One workflow step's text, located by its `- name:` label instead of by a verbatim copy
 * of its body.
 *
 * A probe anchored on a whole step's text breaks the next time anyone adds a flag to that
 * step, and it breaks in the worst way: `replace` returns false, the probe reports
 * "mutation did not apply", and the coverage it was guarding is quietly gone. Two probes
 * here lost exactly that way when the build steps gained a flag. Anchoring on the label —
 * the one part of a step that names what it *is* rather than how it is currently spelled —
 * survives edits to the body.
 *
 * Returns null when no matching step builds, so a probe fails loudly rather than mutating
 * nothing.
 */
function stepByName(dir: string, rel: string, label: RegExp): string | null {
  const lines = readFileSync(join(dir, rel), 'utf8').split('\n')

  for (let start = 0; start < lines.length; start++) {
    if (!/^\s*-\s+name:/.test(lines[start])) continue
    const value = lines[start].replace(/^[^:]*:\s*/, '').replace(/^["']|["']$/g, '').trim()
    if (!label.test(value)) continue

    // The step runs until the next list item at its own indent. Deeper lines belong to it;
    // blanks are kept so the returned text still matches the file byte for byte.
    const indent = indentOf(lines[start])
    let end = start + 1
    while (end < lines.length && (lines[end].trim() === '' || indentOf(lines[end]) > indent)) {
      end++
    }
    const block = lines.slice(start, end).join('\n')

    // Several steps carry "musl" in their label and only one of them invokes napi. A
    // probe handed that step would reformat something real, report its mutation as
    // applied, and pass without ever exercising the checker — the exact silent-coverage
    // loss this helper exists to prevent. Keep looking until the step actually builds.
    if (/napi[ \t]+build/.test(block)) return block
  }
  return null
}

/**
 * Shift every non-blank line of a step two spaces deeper, and drop `token` from it.
 *
 * Returns the block unchanged when the token is absent, so the caller can detect a no-op
 * mutation rather than reporting a pass from an edit that did nothing.
 */
function reindentWithout(block: string, token: string, n = 2): string {
  return block
    .split('\n')
    .map((line) => (line.trim() === '' ? line : ' '.repeat(n) + line.replace(token, '')))
    .join('\n')
}

/**
 * Chain a bare second `napi build` onto the step, immediately before its `--pipe` line.
 *
 * The second invocation is the whole point: a checker that reads a step as one chunk
 * takes the first invocation's flags for both and reports the step as agreeing. Placed
 * after the flag lines deliberately — a second build inserted before them would inherit
 * them from the folded continuation lines and prove nothing.
 */
function withBareSecondBuild(block: string): string {
  const lines = block.split('\n')
  const build = lines.findIndex((line) => /napi[ \t]+build/.test(line))
  const pipe = lines.findIndex((line) => line.trim().startsWith('--pipe'))
  if (build === -1 || pipe === -1) return block
  const cmd = `${' '.repeat(indentOf(lines[build]))}&& pnpm exec napi build --platform --release`
  lines.splice(pipe, 0, cmd)
  return lines.join('\n')
}

describe('verify-feature-parity: the unmodified tree agrees', () => {
  test('exits zero', () => {
    const { code, out } = runChecker(ROOT)
    expect(code, out).toBe(0)
    expect(out).toContain('formula-eval')
  })
})

// `runChecker(ROOT)` above passes because ROOT's own scripts/ copy is used.

describe('verify-feature-parity: a dropped flag is caught', () => {
  test('local build script missing --features', () => {
    expectRejected(
      (d) => replace(d, 'package.json', ' --features formula-eval', '', false),
      'package.json script "build"',
    )
  })

  test('CI build step missing --features', () => {
    expectRejected((d) => replace(d, CI, '          --features formula-eval\n', ''), 'ci.yml')
  })

  test('release musl build step missing --features', () => {
    expectRejected(
      (d) =>
        replace(
          d,
          REL,
          '            --features formula-eval --js native.js --dts native.d.ts \\\n',
          '            --js native.js --dts native.d.ts \\\n',
        ),
      'musl',
    )
  })

  test('every invocation missing --features is degenerate, not agreement', () => {
    expectRejected(
      (d) =>
        replace(d, 'package.json', ' --features formula-eval', '', true) &&
        replace(d, CI, '--features formula-eval', '', true) &&
        replace(d, REL, '--features formula-eval', '', true),
      'declares no',
    )
  })
})

describe('verify-feature-parity: a vanished source is not agreement', () => {
  test('release.yml deleted', () => {
    expectRejected((d) => rmOne(d, REL), 'release.yml')
  })

  test('ci.yml deleted', () => {
    expectRejected((d) => rmOne(d, CI), 'ci.yml')
  })

  test('release.yml moved where readdir cannot see it', () => {
    expectRejected((d) => {
      const nested = join(d, '.github', 'workflows', 'nested')
      mkdirSync(nested, { recursive: true })
      cpSync(join(d, REL), join(nested, 'release.yml'))
      rmOne(d, REL)
      return true
    }, 'release.yml')
  })
})

describe('verify-feature-parity: step boundaries do not depend on indentation', () => {
  test('a re-indented build step cannot borrow a neighbour features', () => {
    // STEP_RE matches one-or-more indent, so shifting a step two spaces deeper must not
    // change how the checker finds it. The step declares no features of its own and the
    // checker must still see that, rather than reading them off a neighbouring chunk it
    // should never have been merged with.
    expectRejected((d) => {
      const block = stepByName(d, REL, /musl/)
      if (block === null) return false
      const next = reindentWithout(block, '--features formula-eval')
      if (next === block) return false
      return replace(d, REL, block, next)
    }, 'musl')
  })
})

describe('verify-feature-parity: each build is compared on its own', () => {
  test('a second napi build in the same step cannot inherit the first flags', () => {
    expectRejected((d) => {
      const block = stepByName(d, REL, /non-musl/)
      if (block === null) return false
      const next = withBareSecondBuild(block)
      if (next === block) return false
      return replace(d, REL, block, next)
    }, 'non-musl')
  })
})

describe('verify-feature-parity: legitimate reformattings still pass', () => {
  test('a backslash-continued --features value', () => {
    expectAccepted((d) =>
      replace(
        d,
        REL,
        '            --features formula-eval --js native.js --dts native.d.ts \\\n',
        '            --features \\\n            formula-eval --js native.js --dts native.d.ts \\\n',
      ),
    )
  })

  test('a quoted --features= form in package.json', () => {
    expectAccepted((d) =>
      replace(d, 'package.json', '--features formula-eval', '--features=\\"formula-eval\\"', true),
    )
  })

  test('an extra feature added consistently to every source', () => {
    expectAccepted(
      (d) =>
        replace(d, 'package.json', '--features formula-eval', '--features formula-eval,extra', true) &&
        replace(d, CI, '--features formula-eval', '--features formula-eval,extra', true) &&
        replace(d, REL, '--features formula-eval', '--features formula-eval,extra', true),
    )
  })

  test('an unrelated new workflow that builds nothing', () => {
    expectAccepted((d) => {
      writeFileSync(
        join(d, '.github', 'workflows', 'unrelated.yml'),
        'name: Unrelated\non: push\njobs:\n  a:\n    runs-on: ubuntu-latest\n' +
          '    steps:\n      - run: echo hi\n',
      )
      return true
    })
  })
})

describe('verify-feature-parity: type-declaration flags', () => {
  // The drift this guards: three workflow steps omitted the flags, so the pipeline
  // generated `const enum` declarations while the local build emitted plain enums.
  // Nothing compared them, and no check read declaration form until
  // verify-build-output.cjs started — which then failed every CI leg.

  test('CI build step missing a type-declaration flag', () => {
    expectRejected((d) => replace(d, CI, '--no-const-enum ', '', false), 'ci.yml')
  })

  test('release musl build step missing a type-declaration flag', () => {
    // `replace_all` is deliberately false: the musl step comes first in release.yml, so
    // this drops the flag from musl only and leaves the non-musl step correct. That
    // proves the checker compares each invocation on its own rather than seeing one
    // correct step speak for the file.
    expectRejected((d) => replace(d, REL, '--no-const-enum ', '', false), 'musl')
  })

  test('a workflow declaring a flag the package.json scripts do not', () => {
    // The comparison is anchored on package.json in both directions. Removing the flag
    // from the reference makes every workflow that still carries it "extra", which is
    // how a workflow drifting ahead of the declared intent is caught rather than
    // silently adopted as the new expectation.
    expectRejected((d) => replace(d, 'package.json', ' --no-const-enum', '', true), 'ci.yml')
  })

  test('a backslash-continued type-declaration flag beside --target and --cross-compile', () => {
    // The musl step carries --cross-compile and --target, neither of which is a
    // type-declaration flag. They must be ignored by construction rather than by an
    // exclusion rule, and the flatten step must still see the flag across a `\` break.
    expectAccepted((d) =>
      replace(
        d,
        REL,
        '            --no-const-enum --runtime-string-enum \\\n',
        '            --no-const-enum \\\n            --runtime-string-enum \\\n',
      ),
    )
  })
})

function rmOne(dir: string, rel: string): boolean {
  rmSync(join(dir, rel), { force: true })
  return true
}

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

/** Copy the files the checker reads into a scratch tree. */
function sandbox(): string {
  const dir = mkdtempSync(join(tmpdir(), 'parity-'))
  cpSync(join(ROOT, '.github'), join(dir, '.github'), { recursive: true })
  cpSync(join(ROOT, 'scripts'), join(dir, 'scripts'), { recursive: true })
  cpSync(join(ROOT, 'package.json'), join(dir, 'package.json'))
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

const MUSL_BUILD_STEP =
  '      - name: "Build native addon (musl: cargo-zigbuild + zig lld)"\n' +
  '        if: ${{ matrix.musl }}\n' +
  '        run: |\n' +
  '          T=${{ matrix.target }}\n' +
  '          export RUSTFLAGS="-C target-feature=-crt-static -C lto=off"\n' +
  '          export RUSTFLAGS="$RUSTFLAGS -C linker-plugin-lto=off"\n' +
  '          export RUSTFLAGS="$RUSTFLAGS -D warnings -C link-arg=-s"\n' +
  '          pnpm exec napi build --cross-compile \\\n' +
  '            --platform --release --target "$T" \\\n' +
  '            --features formula-eval --js native.js --dts native.d.ts \\\n' +
  '            --pipe "node scripts/apply-glue.cjs"\n'

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
    // The original bug: STEP_RE matched only 6-space steps, so an 8-space step
    // merged into the preceding `cargo test --features formula-eval` chunk and
    // inherited its feature set — reported OK while the build declared nothing.
    expectRejected((d) => {
      const ok = replace(
        d,
        REL,
        MUSL_BUILD_STEP,
        MUSL_BUILD_STEP
          .replace('      - name:', '        - name:')
          .replace('        if:', '          if:')
          .replace('        run: |', '          run: |')
          .replace('          T=', '            T=')
          .replace('          export', '            export')
          .replace('          pnpm exec napi build', '            pnpm exec napi build')
          .replace('            --platform', '              --platform')
          .replace('            --features formula-eval', '              --js native.js')
          .replace('            --pipe', '              --pipe'),
      )
      return ok
    }, 'musl')
  })
})

describe('verify-feature-parity: each build is compared on its own', () => {
  test('a second napi build in the same step cannot inherit the first flags', () => {
    expectRejected(
      (d) =>
        replace(
          d,
          REL,
          '          pnpm exec napi build --platform --release\n' +
            '          --target ${{ matrix.target }} --features formula-eval --js native.js\n' +
            '          --dts native.d.ts\n',
          '          pnpm exec napi build --platform --release --target ${{ matrix.target }}\n' +
            '          --features formula-eval --js native.js\n' +
            '          && pnpm exec napi build --platform --release --target ${{ matrix.target }}\n' +
            '          --dts native.d.ts\n',
        ),
      'non-musl',
    )
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

function rmOne(dir: string, rel: string): boolean {
  rmSync(join(dir, rel), { force: true })
  return true
}

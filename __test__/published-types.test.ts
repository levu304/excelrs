import { describe, expect, test } from 'vitest'
import { mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join, resolve } from 'node:path'
import { createRequire } from 'node:module'

// Proves the published type declarations stay importable by a TypeScript consumer
// that transpiles per-file. `declare const enum` is an *ambient* const enum, so a
// consumer compiling with isolatedModules — the default under Vite, Next.js,
// SvelteKit, ts-jest and swc/esbuild — cannot import it at all:
//
//   TS2748: Cannot access ambient const enums when 'isolatedModules' is enabled.
//
// This project's own tsconfig omits isolatedModules, so `pnpm typecheck` stays green
// while the published package is unusable under those toolchains. That is why the
// regression is pinned here rather than left to the repo's own typecheck.
//
// Each probe compiles a scratch consumer against a sandboxed copy of the published
// declarations, so the mutation cannot leak into the working tree.

const ROOT = resolve(__dirname, '..')
const INDEX_DTS = join(ROOT, 'index.d.ts')

// Loaded through createRequire rather than a bare `import` so the bundler does not
// try to transform and source-map the whole compiler bundle on every run.
const ts: typeof import('typescript') = createRequire(import.meta.url)('typescript')

// Every enum index.d.ts declares. Derived from the file rather than hard-coded, so a
// new enum is covered by this probe the day it is added instead of silently escaping.
function publishedEnums(): Array<{ name: string; member: string }> {
  const src = readFileSync(INDEX_DTS, 'utf8')
  const out: Array<{ name: string; member: string }> = []
  const re = /^export declare (?:const )?enum (\w+) \{([\s\S]*?)\n\}/gm
  let m: RegExpExecArray | null
  while ((m = re.exec(src)) !== null) {
    const member = /^\s*(\w+)\s*=/m.exec(m[2])
    if (member) out.push({ name: m[1], member: member[1] })
  }
  return out
}

/**
 * Copy the published declarations into a scratch tree with a consumer beside them,
 * then type-check that consumer with isolatedModules enabled.
 *
 * Returns the diagnostic codes so a probe can assert on TS2748 specifically rather
 * than on "it broke", which would also pass on an unrelated syntax error.
 */
function compileConsumerWithIsolatedModules(
  declarations: string,
  consumerBody: string,
): number[] {
  const dir = mkdtempSync(join(tmpdir(), 'published-types-'))
  try {
    writeFileSync(join(dir, 'index.d.ts'), declarations)
    writeFileSync(join(dir, 'consumer.ts'), consumerBody)
    const program = ts.createProgram([join(dir, 'consumer.ts')], {
      target: ts.ScriptTarget.ES2022,
      module: ts.ModuleKind.ESNext,
      moduleResolution: ts.ModuleResolutionKind.Bundler,
      isolatedModules: true,
      noEmit: true,
      skipLibCheck: true,
      strict: true,
    })
    return [...new Set(ts.getPreEmitDiagnostics(program).map((d) => d.code))]
  } finally {
    rmSync(dir, { recursive: true, force: true })
  }
}

describe('published type declarations under isolatedModules', () => {
  const enums = publishedEnums()

  test('the probe covers every published enum', () => {
    // Guards the probe itself: if the extraction regex broke, every assertion below
    // would vacuously pass on an empty list.
    expect(enums.length).toBeGreaterThan(0)
    expect(enums.map((e) => e.name)).toContain('FillKind')
  })

  test('a consumer can import every published enum and use a member as a value', () => {
    const declarations = readFileSync(INDEX_DTS, 'utf8')
    const body =
      enums
        .map((e) => `import { ${e.name} } from './index.js'\nexport const v_${e.name} = ${e.name}.${e.member}`)
        .join('\n') + '\n'

    const codes = compileConsumerWithIsolatedModules(declarations, body)

    expect(codes).not.toContain(2748)
    expect(codes).toEqual([])
  })

  test('the probe fails when a published enum is declared const', () => {
    // "A check that cannot fail proves nothing" — this is the negative control. It
    // re-introduces the exact defect the probe exists to catch, in a sandbox, and
    // asserts the probe notices.
    const declarations = readFileSync(INDEX_DTS, 'utf8')
    const mutated = declarations.replace(
      /^export declare enum FillKind \{/m,
      'export declare const enum FillKind {',
    )
    expect(mutated).not.toBe(declarations) // the mutation applied

    const codes = compileConsumerWithIsolatedModules(
      mutated,
      "import { FillKind } from './index.js'\nexport const v = FillKind.Solid\n",
    )

    expect(codes).toContain(2748)
  })
})

describe('WorksheetState stays assignable from bare string literals', () => {
  const LITERAL_CONSUMER =
    "import { WorksheetState } from './index.js'\n" +
    "declare const ws: { state: WorksheetState }\n" +
    "ws.state = 'visible'\n" +
    "export const s: WorksheetState = 'hidden'\n"

  test('a consumer can assign the bare literals', () => {
    // WorksheetState aliases SheetState, but it must NOT be the enum type itself: a
    // string enum rejects bare string literals, so `ws.state = 'visible'` would stop
    // compiling. The alias is therefore derived as a template literal type.
    const codes = compileConsumerWithIsolatedModules(readFileSync(INDEX_DTS, 'utf8'), LITERAL_CONSUMER)

    expect(codes).toEqual([])
  })

  test('the probe fails when the alias points straight at the enum', () => {
    // Negative control for the trap above: the obvious "just alias it" implementation
    // compiles the declaration but breaks every consumer that assigns a literal.
    const declarations = readFileSync(INDEX_DTS, 'utf8')
    const mutated = declarations.replace(
      /^export type WorksheetState = `\$\{SheetState\}`$/m,
      'export type WorksheetState = SheetState',
    )
    expect(mutated).not.toBe(declarations) // the mutation applied

    const codes = compileConsumerWithIsolatedModules(mutated, LITERAL_CONSUMER)

    expect(codes).toContain(2322)
  })
})

describe('published enum members resolve at runtime', () => {
  test('an imported enum member equals its declared string value', async () => {
    // A plain `enum` needs the entrypoint to actually supply the object; a `const enum`
    // needs nothing at runtime. Removing `const` therefore moves a requirement onto
    // index.js, and this is what holds it there.
    const { FillKind, CellType, SheetViewState } = await import('../index.js')

    expect(FillKind.Solid).toBe('Solid')
    expect(FillKind.None).toBe('None')
    expect(CellType.Number).toBe('Number')
    expect(SheetViewState.Frozen).toBe('Frozen')
  })

  test('every published enum is reachable from the entrypoint', async () => {
    const binding = await import('../index.js')
    const missing = publishedEnums()
      .map((e) => e.name)
      .filter((name) => !(name in binding))

    expect(missing).toEqual([])
  })
})

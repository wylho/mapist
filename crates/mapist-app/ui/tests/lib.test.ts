// Testes das funções puras do frontend: temas, medição e geometria.
//
// Só o que não precisa de janela. O canvas em si é exercitado pelo app.

import { describe, expect, it, beforeAll } from 'vitest'
import {
  contrast,
  cssVars,
  DESKTOPS,
  fitAccent,
  mix,
  NODE_COLOR_TOKENS,
  PALETTES,
  paletteColors,
  resolveNodeColor,
  systemColors,
} from '../src/lib/palettes'
import { hitTest, resolveColor } from '../src/lib/render'
import type { Snapshot } from '../src/lib/api'

describe('mix', () => {
  it('returns the first colour at t=1 and the second at t=0', () => {
    expect(mix('#ffffff', '#000000', 1)).toBe('#ffffff')
    expect(mix('#ffffff', '#000000', 0)).toBe('#000000')
  })

  it('blends halfway', () => {
    expect(mix('#ffffff', '#000000', 0.5)).toBe('#808080')
  })

  it('accepts three-digit hex', () => {
    expect(mix('#fff', '#000', 1)).toBe('#ffffff')
  })
})

describe('contrast', () => {
  it('is highest between black and white', () => {
    expect(contrast('#ffffff', '#000000')).toBeCloseTo(21, 0)
  })

  it('is symmetric', () => {
    expect(contrast('#123456', '#abcdef')).toBeCloseTo(contrast('#abcdef', '#123456'), 10)
  })

  it('is 1 for a colour against itself', () => {
    expect(contrast('#4c9aff', '#4c9aff')).toBeCloseTo(1, 10)
  })
})

describe('fitAccent', () => {
  it('leaves an already-legible accent alone', () => {
    // A dark blue on white is fine as it is.
    expect(fitAccent('#2547c9', 'light', '#ffffff')).toBe('#2547c9')
  })

  it('darkens a too-light accent in light mode', () => {
    // GNOME's yellow is unreadable on white; this is the case the Ideario added it for.
    const out = fitAccent('#c88800', 'light', '#ffffff')
    expect(out).not.toBe('#c88800')
    expect(contrast(out, '#ffffff')).toBeGreaterThanOrEqual(3.2)
  })

  it('lightens a too-dark accent in dark mode', () => {
    const out = fitAccent('#101010', 'dark', '#19211e')
    expect(contrast(out, '#19211e')).toBeGreaterThanOrEqual(3.2)
  })

  it('reaches the target for every palette accent', () => {
    for (const [id, p] of Object.entries(PALETTES)) {
      for (const mode of ['light', 'dark'] as const) {
        const c = p[mode]
        const fitted = fitAccent(c.accent, mode, c.surface)
        expect(contrast(fitted, c.surface), `${id}/${mode}`).toBeGreaterThanOrEqual(3.2)
      }
    }
  })
})

describe('palettes', () => {
  it('defines every palette in both modes', () => {
    for (const p of Object.values(PALETTES)) {
      expect(p.light.bg).toMatch(/^#[0-9a-f]{6}$/i)
      expect(p.dark.bg).toMatch(/^#[0-9a-f]{6}$/i)
      expect(p.light.notes).toHaveLength(6)
      expect(p.dark.notes).toHaveLength(6)
    }
  })

  it('defines every desktop in both modes', () => {
    for (const d of Object.values(DESKTOPS)) {
      expect(d.light.notes).toHaveLength(6)
      expect(d.dark.notes).toHaveLength(6)
      expect(d.accent).toMatch(/^#[0-9a-f]{6}$/i)
    }
  })

  it('resolves the system theme through the desktop palette', () => {
    const c = systemColors('gnome', 'light', null)
    expect(c.bg).toBe(DESKTOPS.gnome.light.bg)
    expect(c.accent).not.toBe('')
  })

  it('uses a supplied system accent, adjusted for legibility', () => {
    const c = systemColors('gnome', 'light', '#c88800')
    expect(contrast(c.accent, c.surface)).toBeGreaterThanOrEqual(3.2)
  })

  it('routes paletteColors through both paths', () => {
    expect(paletteColors('papel', 'dark', 'gnome', null).bg).toBe(PALETTES.papel.dark.bg)
    expect(paletteColors('system', 'light', 'kde', null).bg).toBe(DESKTOPS.kde.light.bg)
  })
})

describe('cssVars', () => {
  it('emits every variable app.css expects', () => {
    const v = cssVars(PALETTES.ideario.light, 'light')
    for (const key of [
      '--bg',
      '--stage',
      '--surface',
      '--raised',
      '--fg',
      '--muted',
      '--line',
      '--card-edge',
      '--accent',
      '--accent-soft',
      '--on-accent',
      '--shadow',
      '--float-shadow',
      '--n-sand',
      '--n-sage',
      '--n-sky',
      '--n-rose',
      '--n-lilac',
      '--n-butter',
    ]) {
      expect(v[key], key).toBeTruthy()
    }
  })

  it('derives accent-soft when the palette omits it', () => {
    const noSoft = { ...PALETTES.ideario.light, accentSoft: undefined }
    const v = cssVars(noSoft, 'light')
    expect(v['--accent-soft']).toBeTruthy()
    expect(v['--accent-soft']).not.toBe(noSoft.accent)
  })

  it('chooses readable text on the accent', () => {
    const v = cssVars(PALETTES.grafite.dark, 'dark')
    expect(v['--on-accent']).toMatch(/^#[0-9a-f]{6}$/i)
  })

  it('gives dark mode a dark shadow', () => {
    const v = cssVars(PALETTES.ideario.dark, 'dark')
    expect(v['--shadow']).toContain('rgb(0 0 0')
  })
})

describe('hitTest', () => {
  const snapshot: Snapshot = {
    nodes: [
      {
        id: 0,
        text: 'Root',
        note: '',
        side: 'right',
        depth: 0,
        x: 0,
        y: 0,
        width: 100,
        height: 40,
        color: null,
        folded: false,
        child_ids: [1],
        parent: null,
        has_children: true,
      },
      {
        id: 1,
        text: 'Child',
        note: '',
        side: 'right',
        depth: 1,
        x: 200,
        y: 0,
        width: 80,
        height: 40,
        color: null,
        folded: false,
        child_ids: [],
        parent: 0,
        has_children: false,
      },
    ],
    connections: [],
    dirty: false,
    path: null,
    title: 'Root',
    can_undo: false,
    can_redo: false,
    bounds: { min_x: 0, min_y: 0, max_x: 280, max_y: 40 },
  }
  const rect = { left: 0, top: 0 } as DOMRect
  const view = { x: 0, y: 0, scale: 1 }

  it('finds a node under the point', () => {
    expect(hitTest(snapshot, view, 50, 20, rect)).toBe(0)
    expect(hitTest(snapshot, view, 240, 20, rect)).toBe(1)
  })

  it('returns null in empty space', () => {
    expect(hitTest(snapshot, view, 500, 500, rect)).toBeNull()
  })

  it('respects the viewport offset', () => {
    const shifted = { x: 100, y: 50, scale: 1 }
    expect(hitTest(snapshot, view, 50, 20, rect)).toBe(0)
    expect(hitTest(snapshot, shifted, 150, 70, rect)).toBe(0)
    expect(hitTest(snapshot, shifted, 50, 20, rect)).toBeNull()
  })

  it('respects zoom', () => {
    const zoomed = { x: 0, y: 0, scale: 2 }
    expect(hitTest(snapshot, zoomed, 100, 40, rect)).toBe(0)
    // 100 world units wide at 2x ends at 200 screen px.
    expect(hitTest(snapshot, zoomed, 250, 40, rect)).toBeNull()
  })

  it('prefers the topmost node when they overlap', () => {
    const overlapping: Snapshot = {
      ...snapshot,
      nodes: [snapshot.nodes[0], { ...snapshot.nodes[1], x: 0, y: 0 }],
    }
    expect(hitTest(overlapping, view, 50, 20, rect)).toBe(1)
  })
})

describe('cor de nó: token vira hex, nunca o contrário', () => {
  it('resolve cada token para um hex válido do tema', () => {
    const c = PALETTES.ideario.light
    for (const t of NODE_COLOR_TOKENS) {
      expect(resolveNodeColor(t, c), t).toMatch(/^#[0-9a-f]{6}$/i)
    }
  })

  it('resolve o token para o primeiro valor de notes, na ordem certa', () => {
    const c = PALETTES.ideario.light
    expect(resolveNodeColor('--n-sand', c)).toBe(NOTES_FIRST)
    expect(resolveNodeColor('--n-sand', c)).toBe(c.notes[0])
  })

  it('passa hex direto, como um .minder carregaria', () => {
    const c = PALETTES.ideario.light
    expect(resolveNodeColor('#c6262e', c)).toBe('#c6262e')
  })

  it('resolveColor do render concorda com resolveNodeColor', () => {
    const colors = {
      stage: '',
      surface: '#ffffff',
      raised: '',
      fg: '',
      muted: '',
      line: '',
      accent: '',
      onAccent: '',
      cardEdge: '',
      nodeColors: PALETTES.ideario.light.notes,
    }
    for (const t of NODE_COLOR_TOKENS) {
      expect(resolveColor(t, colors)).toBe(resolveNodeColor(t, PALETTES.ideario.light))
    }
    expect(resolveColor('#c6262e', colors)).toBe('#c6262e')
    expect(resolveColor(null, colors)).toBe('#ffffff')
  })
})

const NOTES_FIRST = PALETTES.ideario.light.notes[0]

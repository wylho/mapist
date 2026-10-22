// Testes das funções puras do frontend: temas, medição e geometria.
//
// Só o que não precisa de janela. O canvas em si é exercitado pelo app.

import { describe, expect, it, beforeAll, vi } from 'vitest'
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
import { hitTest, resolveColor, traceBranch } from '../src/lib/render'
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
    const v = cssVars(PALETTES.atrio.light, 'light')
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
    const noSoft = { ...PALETTES.atrio.light, accentSoft: undefined }
    const v = cssVars(noSoft, 'light')
    expect(v['--accent-soft']).toBeTruthy()
    expect(v['--accent-soft']).not.toBe(noSoft.accent)
  })

  it('chooses readable text on the accent', () => {
    const v = cssVars(PALETTES.grafite.dark, 'dark')
    expect(v['--on-accent']).toMatch(/^#[0-9a-f]{6}$/i)
  })

  it('gives dark mode a dark shadow', () => {
    const v = cssVars(PALETTES.atrio.dark, 'dark')
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
    notes: [],
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
    const c = PALETTES.atrio.light
    for (const t of NODE_COLOR_TOKENS) {
      expect(resolveNodeColor(t, c), t).toMatch(/^#[0-9a-f]{6}$/i)
    }
  })

  it('resolve o token para o primeiro valor de notes, na ordem certa', () => {
    const c = PALETTES.atrio.light
    expect(resolveNodeColor('--n-sand', c)).toBe(NOTES_FIRST)
    expect(resolveNodeColor('--n-sand', c)).toBe(c.notes[0])
  })

  it('passa hex direto, como um .minder carregaria', () => {
    const c = PALETTES.atrio.light
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
      nodeColors: PALETTES.atrio.light.notes,
    }
    for (const t of NODE_COLOR_TOKENS) {
      expect(resolveColor(t, colors)).toBe(resolveNodeColor(t, PALETTES.atrio.light))
    }
    expect(resolveColor('#c6262e', colors)).toBe('#c6262e')
    expect(resolveColor(null, colors)).toBe('#ffffff')
  })
})

const NOTES_FIRST = PALETTES.atrio.light.notes[0]

describe('traceBranch', () => {
  function recorder() {
    const calls: string[] = []
    return {
      calls,
      ctx: {
        moveTo: (x: number, y: number) => calls.push(`M ${x} ${y}`),
        lineTo: (x: number, y: number) => calls.push(`L ${x} ${y}`),
        bezierCurveTo: (...a: number[]) => calls.push(`C ${a.join(' ')}`),
        arcTo: (...a: number[]) => calls.push(`A ${a.join(' ')}`),
      },
    }
  }

  it('curvas: uma Bézier com controles no meio', () => {
    const r = recorder()
    traceBranch(r.ctx, 'curved', 0, 0, 100, 60)
    expect(r.calls).toEqual(['M 0 0', 'C 50 0 50 60 100 60'])
  })

  it('em ângulo: degrau com dois cantos arredondados que termina no filho', () => {
    const r = recorder()
    traceBranch(r.ctx, 'orthogonal', 0, 0, 100, 60)
    expect(r.calls).toEqual(['M 0 0', 'A 50 0 50 60 14', 'A 50 60 100 60 14', 'L 100 60'])
  })

  it('em ângulo: o canto encolhe com desnível pequeno e vira reta quando alinhado', () => {
    const small = recorder()
    traceBranch(small.ctx, 'orthogonal', 0, 0, 100, 10)
    expect(small.calls[1]).toBe('A 50 0 50 10 5')

    const flat = recorder()
    traceBranch(flat.ctx, 'orthogonal', 0, 20, 100, 20)
    expect(flat.calls).toEqual(['M 0 20', 'L 100 20'])
  })

  it('em ângulo: funciona para o lado esquerdo (x decrescente)', () => {
    const r = recorder()
    traceBranch(r.ctx, 'orthogonal', 0, 0, -100, -60)
    expect(r.calls.at(-1)).toBe('L -100 -60')
  })
})

describe('preview: layout espelha o núcleo', () => {
  it('ramo da esquerda fica à esquerda da raiz, sem nós sobrepostos, e recolhido esconde os filhos', async () => {
    const { seedDemo, previewBackend } = await import('../src/lib/preview')
    seedDemo()
    let snap = previewBackend.get_snapshot()
    const research = snap.nodes.find((n) => n.text === 'Pesquisa')!
    expect(research.folded).toBe(true)
    expect(snap.nodes.some((n) => n.parent === research.id)).toBe(false)

    snap = previewBackend.toggle_fold({ id: research.id })
    const root = snap.nodes[0]
    const kids = snap.nodes.filter((n) => n.parent === research.id)
    expect(kids.length).toBe(2)
    const r = snap.nodes.find((n) => n.id === research.id)!
    expect(r.x + r.width).toBeLessThanOrEqual(root.x)
    for (const k of kids) expect(k.x + k.width).toBeLessThanOrEqual(r.x)

    for (const a of snap.nodes)
      for (const b of snap.nodes) {
        if (a.id >= b.id) continue
        const overlap = a.x < b.x + b.width && b.x < a.x + a.width && a.y < b.y + b.height && b.y < a.y + a.height
        expect(overlap, `${a.text} × ${b.text}`).toBe(false)
      }
  })
})

describe('edição estilo XMind', () => {
  it('raiz selecionada ao abrir, Tab cria filho já em edição, Enter na raiz cria filho, texto vazio não apaga', async () => {
    const { doc } = await import('../src/lib/document.svelte')
    await doc.create('Raiz')
    const root = doc.snapshot.nodes[0]
    expect(doc.selected).toBe(root.id)

    doc.startEdit('P')
    expect(doc.editing).toMatchObject({ id: root.id, draft: 'P', selectAll: false })
    doc.editing!.draft = 'Plano'
    await doc.commitEdit()
    expect(doc.snapshot.nodes[0].text).toBe('Plano')

    await doc.addChild()
    const child = doc.selectedNode!
    expect(child.parent).toBe(root.id)
    expect(doc.editing).toMatchObject({ id: child.id, draft: 'Nova ideia', selectAll: true })
    doc.editing!.draft = '   '
    await doc.commitEdit()
    expect(doc.node(child.id)!.text).toBe('Nova ideia')

    doc.selected = root.id
    await doc.addSibling()
    expect(doc.selectedNode!.parent).toBe(root.id)
    doc.cancelEdit()
    expect(doc.editing).toBeNull()
  })

  it('o que se digita enquanto o nó novo nasce vai para ele, e não para o que estava selecionado', async () => {
    const { doc } = await import('../src/lib/document.svelte')
    await doc.create('Raiz')
    const root = doc.snapshot.nodes[0]

    // Enter/Tab e já sair digitando: as letras chegam antes de o nó existir.
    const creating = doc.addChild()
    doc.type('O')
    doc.type('i')
    await creating
    const child = doc.selectedNode!
    expect(child.id).not.toBe(root.id)
    expect(doc.editing).toMatchObject({ id: child.id, draft: 'Oi', selectAll: false })

    // Campo já aberto, mas o foco ainda não chegou nele: a letra entra no rascunho.
    doc.type('!')
    expect(doc.editing!.draft).toBe('Oi!')
    await doc.commitEdit()
    expect(doc.node(child.id)!.text).toBe('Oi!')
    expect(doc.node(root.id)!.text).toBe('Raiz')
  })

  it('sem nada digitado, o nó novo abre com o texto provisório selecionado', async () => {
    const { doc } = await import('../src/lib/document.svelte')
    await doc.create('Raiz')
    await doc.addChild()
    expect(doc.editing).toMatchObject({ draft: 'Nova ideia', selectAll: true })
    doc.type('X')
    expect(doc.editing).toMatchObject({ draft: 'X', selectAll: false })
    doc.cancelEdit()
  })
})

describe('atalhos de estrutura', () => {
  it('⌥↑/⌥↓ sobem e descem o tópico entre os irmãos; Home seleciona a raiz', async () => {
    const { doc } = await import('../src/lib/document.svelte')
    await doc.create('Raiz')
    const root = doc.snapshot.nodes[0].id
    await doc.addChild('P')
    doc.cancelEdit()
    const p = doc.selected!
    await doc.addChild('A')
    doc.cancelEdit()
    doc.selected = p
    await doc.addChild('B')
    doc.cancelEdit()
    const b = doc.selected!
    const kids = () => doc.node(p)!.child_ids.map((id) => doc.node(id)!.text)
    expect(kids()).toEqual(['A', 'B'])

    await doc.reorder(-1)
    expect(kids()).toEqual(['B', 'A'])
    expect(doc.selected).toBe(b)
    // Já é o primeiro: fica onde está, sem erro.
    await doc.reorder(-1)
    expect(kids()).toEqual(['B', 'A'])
    await doc.undo()
    expect(kids()).toEqual(['A', 'B'])

    doc.selectRoot()
    expect(doc.selected).toBe(root)
  })
})

describe('estruturas do mapa', () => {
  it('a prévia arruma como o núcleo: organograma para baixo, lógico à direita, árvore em ordem de leitura', async () => {
    const { doc } = await import('../src/lib/document.svelte')
    await doc.create('Raiz')
    const root = doc.snapshot.nodes[0].id
    for (const t of ['A', 'B', 'C']) {
      doc.selected = root
      await doc.addChild(t)
      doc.cancelEdit()
    }
    const byText = (t: string) => doc.snapshot.nodes.find((n) => n.text === t)!
    await doc.setSide(byText('B').id, 'left')

    await doc.setStructure('organograma')
    const r = byText('Raiz')
    for (const t of ['A', 'B', 'C']) expect(byText(t).y).toBeGreaterThanOrEqual(r.y + r.height)
    expect(byText('A').y).toBe(byText('B').y)

    await doc.setStructure('logico')
    for (const t of ['A', 'B', 'C']) expect(byText(t).x).toBeGreaterThanOrEqual(byText('Raiz').x + byText('Raiz').width)

    await doc.setStructure('arvore')
    const ys = ['Raiz', 'A', 'B', 'C'].map((t) => byText(t).y)
    expect([...ys].sort((a, b) => a - b)).toEqual(ys)
    expect(doc.snapshot.structure).toBe('arvore')

    const gap = () => byText('B').y - (byText('A').y + byText('A').height)
    const normal = gap()
    await doc.setSpacing('largo')
    expect(gap()).toBeGreaterThan(normal)

    await doc.setStructure('mapa')
    expect(doc.snapshot.structure).toBeNull()
    expect(byText('B').x + byText('B').width).toBeLessThanOrEqual(byText('Raiz').x)
  })
})

describe('temas de mapa', () => {
  it('cada ramo principal ganha uma cor, os filhos herdam, e a cor escolhida à mão manda', async () => {
    const { branchColors, mapTheme, MAP_THEMES } = await import('../src/lib/maptheme')
    const node = (id: number, depth: number, parent: number | null, color: string | null = null) =>
      ({ id, depth, parent, color, text: '', note: '', side: 'right', x: 0, y: 0, width: 0, height: 0, folded: false, child_ids: [], has_children: false }) as never
    const nodes = [node(0, 0, null), node(1, 1, 0), node(2, 2, 1), node(3, 1, 0, '#123456'), node(4, 2, 3), node(5, 1, 0)]
    const colors = branchColors(nodes, MAP_THEMES.vibrante, 'light', [], (c) => c)
    expect(colors.has(0)).toBe(false)
    expect(colors.get(2)).toBe(colors.get(1))
    expect(colors.get(3)).toBe('#123456')
    expect(colors.get(4)).toBe('#123456')
    expect(new Set([colors.get(1), colors.get(3), colors.get(5)]).size).toBe(3)
    expect(mapTheme('nao-existe').id).toBe('classico')
    expect(mapTheme(null).id).toBe('classico')
  })

  it('os nomes batem com os do núcleo (nodo_core::MAP_STYLES)', async () => {
    const { MAP_THEME_IDS } = await import('../src/lib/maptheme')
    const fs = await import('node:fs')
    const path = await import('node:path')
    // Os testes rodam a partir de crates/nodo-app/ui.
    const core = fs.readFileSync(path.resolve(process.cwd(), '../../nodo-core/src/lib.rs'), 'utf8')
    const list = /MAP_STYLES: \[&str; \d+\] = \[([^\]]*)\]/.exec(core)![1]
    expect([...list.matchAll(/"([^"]+)"/g)].map((m) => m[1])).toEqual(MAP_THEME_IDS)
  })
})

describe('exportação', () => {
  it('o SVG grava o arcTo do canvas como reta + arco com os pontos de tangência certos', async () => {
    const { SvgContext } = await import('../src/lib/svgctx')
    const c = new SvgContext()
    c.beginPath()
    c.moveTo(0, 0)
    c.arcTo(100, 0, 100, 100, 10)
    c.lineTo(100, 100)
    c.strokeStyle = '#123456'
    c.stroke()
    c.font = '600 13px Inter, system-ui'
    c.textAlign = 'center'
    c.textBaseline = 'middle'
    c.fillStyle = '#000'
    c.fillText('Olá & <tudo>', 50, 50)
    const svg = c.toSvg(120, 120, '#fff')
    // Reta até (90, 0), arco no sentido horário até (100, 10).
    expect(svg).toContain('d="M0 0L90 0A10 10 0 0 1 100 10L100 100"')
    expect(svg).toContain('stroke="#123456"')
    expect(svg).toContain('font-weight="600" fill="#000" text-anchor="middle" dominant-baseline="central"')
    expect(svg).toContain('>Olá &amp; &lt;tudo&gt;</text>')
    expect(svg.startsWith('<svg xmlns="http://www.w3.org/2000/svg" width="120" height="120"')).toBe(true)
  })

  it('PDF automático: paisagem para mapa largo, retrato para mapa alto; a escolha manual vale', async () => {
    const { pickOrientation } = await import('../src/lib/exporter')
    const snap = (w: number, h: number) =>
      ({ bounds: { min_x: 0, min_y: 0, max_x: w, max_y: h } }) as unknown as import('../src/lib/api').Snapshot
    expect(pickOrientation(snap(1200, 400), 'auto')).toBe('landscape')
    expect(pickOrientation(snap(300, 1500), 'auto')).toBe('portrait')
    expect(pickOrientation(snap(1200, 400), 'portrait')).toBe('portrait')
  })

  it('a moldura conta a altura real do card da nota solta (ele cresce com o texto)', async () => {
    const { exportFrame } = await import('../src/lib/exporter')
    const s = { bounds: { min_x: 0, min_y: 0, max_x: 500, max_y: 300 }, notes: [{ id: 7, x: 0, y: 200, height: 120 }] } as unknown as import('../src/lib/api').Snapshot
    expect(exportFrame(s).height).toBe(300 + 80)
    const cards = new Map([['f:7', { width: 240, height: 400, ops: [] }]])
    expect(exportFrame(s, cards).height).toBe(600 + 80)
  })
})

describe('notas soltas no mapa', () => {
  it('a prévia cria, edita, move e apaga como o núcleo, e o desfazer volta', async () => {
    const { seedDemo, previewBackend } = await import('../src/lib/preview')
    seedDemo()
    const before = previewBackend.get_snapshot().bounds
    const count = before ? previewBackend.get_snapshot().notes.length : 0
    let snap = previewBackend.add_free_note({ x: before.max_x + 100, y: 0, text: '' })
    const id = snap.notes.at(-1)!.id
    const mine = () => snap.notes.find((n) => n.id === id)
    expect(snap.bounds.max_x).toBe(before.max_x + 100 + 240)
    snap = previewBackend.set_free_note({ id, text: '**Prazo** sexta' })
    snap = previewBackend.place_free_note({ id, x: 10, y: 20, width: 50, height: 300 })
    expect(mine()).toMatchObject({ x: 10, y: 20, width: 120, height: 300, text: '**Prazo** sexta' })
    snap = previewBackend.remove_free_note({ id })
    expect(snap.notes).toHaveLength(count)
    snap = previewBackend.undo()
    expect(mine()?.text).toBe('**Prazo** sexta')
  })

  it('a nota solta é o card padrão: título, cor de nota, campos e a prévia, como um tópico', async () => {
    const { seedDemo, previewBackend } = await import('../src/lib/preview')
    seedDemo()
    let snap = previewBackend.add_free_note({ x: 0, y: 0, text: '- [ ] pão #casa' })
    const id = snap.notes.at(-1)!.id
    snap = previewBackend.set_free_card({ id, title: '  Compras ', color: 'roxo', card: { tags: ['#Feira'], status_on: true } })
    const f = snap.notes.find((n) => n.id === id)!
    expect(f).toMatchObject({ title: 'Compras', color: null, card: { tags: ['feira'], status_on: true } })
    expect(f.view?.preview[0]).toMatchObject({ kind: 'task', text: 'pão #casa' })
    expect(f.view?.hash_tags).toEqual(['casa'])
  })
})

describe('card do Atrio entre o Ideario e o Nodo', () => {
  it('colar um card cria um filho com a nota (um passo do desfazer) e copiar devolve o mesmo card', async () => {
    const { seedDemo } = await import('../src/lib/preview')
    const { doc } = await import('../src/lib/document.svelte')
    const { cardFromHtml, cardToClipboard } = await import('@wylho/atrio')
    seedDemo()
    await doc.load()
    const root = doc.snapshot.nodes.find((n) => n.parent === null)!
    doc.selected = root.id
    const copied = cardFromHtml(
      cardToClipboard({ title: 'Feira', markdown: '- [ ] Arroz #casa', tags: ['casa', '#Mercado'], emoji: '🧺', color: 'sky' }).html,
    )!
    await doc.pasteCard(copied)
    const n = doc.selectedNode!
    expect(n.parent).toBe(root.id)
    expect(n.text).toBe('Feira')
    expect(n.note).toBe('- [ ] Arroz #casa')
    expect(n.card).toEqual({ tags: ['casa', 'mercado'], emoji: '🧺' })
    expect(doc.selectedCard()).toEqual({
      title: 'Feira',
      markdown: '- [ ] Arroz #casa',
      color: 'sky',
      emoji: '🧺',
      tags: ['casa', 'mercado'],
    })
    await doc.undo()
    expect(doc.snapshot.nodes.some((x) => x.id === n.id)).toBe(false)
  })
})

describe('campos do card no nó', () => {
  it('o preview limpa como o núcleo e desfaz num passo', async () => {
    const { previewBackend, seedDemo } = await import('../src/lib/preview')
    seedDemo()
    const id = previewBackend.get_snapshot().nodes.find((n) => n.text === 'Design')!.id
    let snap = previewBackend.set_card({
      id,
      card: { tags: ['#Q3', 'q3', ' '], emoji: '🚀', reminder_repeat: 'week', status_on: true, reminder_doing: true },
    })
    expect(snap.nodes.find((n) => n.id === id)?.card).toEqual({
      tags: ['q3'],
      emoji: '🚀',
      reminder_doing: true,
      status_on: true,
    })
    snap = previewBackend.undo()
    expect(snap.nodes.find((n) => n.id === id)?.card).toEqual({})
  })

  it('a nota vai e volta entre Markdown e o documento do editor', async () => {
    const { docToMarkdown, markdownToDoc } = await import('../src/lib/notedoc')
    const md = '# Plano\n\n- [ ] Roteiro\n- [x] **Convites**\n\n1. Um\n2. Dois\n\nPrazo *sexta*'
    expect(docToMarkdown(markdownToDoc(md))).toBe(md)
  })
})

describe('status pelo card do mapa', () => {
  it('muda com as regras do card e "Sem status" tira o que foi posto à mão', async () => {
    const { seedDemo } = await import('../src/lib/preview')
    const { doc } = await import('../src/lib/document.svelte')
    seedDemo()
    await doc.load()
    const id = doc.snapshot.nodes.find((n) => n.text === 'Design')!.id
    await doc.setStatus(id, 'doing')
    expect(doc.node(id)?.card).toEqual({ reminder_doing: true, status_on: true })
    await doc.setStatus(id, 'done')
    expect(doc.node(id)?.card).toEqual({ reminder_done: true, status_on: true })
    await doc.setStatus(id, 'none')
    expect(doc.node(id)?.card).toEqual({})
  })
})

describe('abas', () => {
  it('novo mapa abre noutra aba, trocar volta ao que estava e fechar com algo por salvar pergunta', async () => {
    const { seedDemo } = await import('../src/lib/preview')
    const { doc } = await import('../src/lib/document.svelte')
    const { ui } = await import('../src/lib/ui.svelte')
    seedDemo()
    await doc.load()
    const first = doc.snapshot.active!
    const demo = doc.snapshot.title
    await doc.newMap()
    expect(doc.tabs).toHaveLength(2)
    expect(doc.snapshot.title).toBe('Tópico central')
    const second = doc.snapshot.active!
    await doc.switchTab(first)
    expect(doc.snapshot.title).toBe(demo)
    await doc.setText(doc.snapshot.nodes[0].id, 'Lançamento revisto')
    expect(doc.tabs.find((t) => t.id === first)?.dirty).toBe(true)

    // Cancelar: a aba fica.
    let closing = doc.closeTab(first)
    await vi.waitFor(() => expect(ui.confirmRequest?.save).toBe('Salvar'))
    const asked = ui.confirmRequest!
    ui.confirmRequest = null
    asked.oncancel!()
    await closing
    expect(doc.tabs).toHaveLength(2)

    // Fechar sem salvar: sai, e a outra vem para a frente.
    closing = doc.closeTab(first)
    await vi.waitFor(() => expect(ui.confirmRequest).not.toBeNull())
    const req = ui.confirmRequest!
    ui.confirmRequest = null
    req.onconfirm()
    await closing
    expect(doc.tabs.map((t) => t.id)).toEqual([second])
    expect(doc.snapshot.active).toBe(second)

    // Fechar a última: um mapa novo no lugar, nunca a janela vazia.
    await doc.closeTab(second)
    expect(doc.tabs).toHaveLength(1)
    expect(doc.hasDocument).toBe(true)
  })

  it('abrir um arquivo já aberto traz a aba dele, sem abrir de novo', async () => {
    const { seedDemo } = await import('../src/lib/preview')
    const { doc } = await import('../src/lib/document.svelte')
    seedDemo()
    await doc.load()
    await doc.openPath('/mapas/viagem.nodo')
    await doc.newMap()
    await doc.openPath('/mapas/viagem.nodo')
    expect(doc.tabs.filter((t) => t.path === '/mapas/viagem.nodo')).toHaveLength(1)
    expect(doc.tabs.find((t) => t.id === doc.snapshot.active)?.path).toBe('/mapas/viagem.nodo')
  })
})

describe('exportação: o card lido da tela', () => {
  it('cor de opacidade zero não é tinta, mas uma cor com azul zero é', async () => {
    const { clear } = await import('../src/lib/domcard')
    expect(['transparent', 'rgba(0, 0, 0, 0)', 'rgba(10, 20, 30, 0.0)', ''].every(clear)).toBe(true)
    expect(['rgb(232, 164, 0)', 'rgba(232, 164, 0, 0.22)', 'rgb(0, 0, 0)'].some(clear)).toBe(false)
  })
})


describe('exportar com ramos abertos', () => {
  it('a prévia abre tudo para a imagem e devolve o mapa exatamente como estava', async () => {
    const { seedDemo, previewBackend } = await import('../src/lib/preview')
    seedDemo()
    const before = previewBackend.get_snapshot()
    expect(before.nodes.some((n) => n.folded)).toBe(true)
    const open = previewBackend.export_layout({ sizes: [], unfold: true })
    expect(open.nodes.length).toBeGreaterThan(before.nodes.length)
    expect(open.nodes.some((n) => n.folded)).toBe(false)
    expect(previewBackend.get_snapshot()).toEqual(before)
  })
})

describe('estruturas de tabela', () => {
  for (const structure of ['matriz', 'tabela']) {
    it(`${structure}: cada tópico na sua célula, sem células sobrepostas, como o núcleo`, async () => {
      const { seedDemo, previewBackend } = await import('../src/lib/preview')
      seedDemo()
      const s = previewBackend.set_structure({ structure })
      const cells = s.nodes.filter((n) => n.cell)
      expect(cells.length).toBeGreaterThan(5)
      for (const n of cells) {
        const [x, y, w, h] = n.cell!
        expect(n.x >= x && n.y >= y && n.x + n.width <= x + w + 1e-6 && n.y + n.height <= y + h + 1e-6).toBe(true)
      }
      for (const [i, a] of cells.entries())
        for (const b of cells.slice(i + 1)) {
          const [ax, ay, aw, ah] = a.cell!
          const [bx, by, bw, bh] = b.cell!
          expect(ax < bx + bw - 1e-6 && bx < ax + aw - 1e-6 && ay < by + bh - 1e-6 && by < ay + ah - 1e-6, `${a.text} × ${b.text}`).toBe(false)
        }
      expect(s.bounds.max_x).toBeGreaterThanOrEqual(Math.max(...cells.map((n) => n.cell![0] + n.cell![2])) - 1e-6)
    })
  }
})

describe('linha do tempo', () => {
  it('principais na mesma linha e em ordem; galhos alternando acima e abaixo; nada se sobrepõe', async () => {
    const { seedDemo, previewBackend } = await import('../src/lib/preview')
    seedDemo()
    previewBackend.toggle_fold({ id: previewBackend.get_snapshot().nodes.find((n) => n.folded)!.id })
    const s = previewBackend.set_structure({ structure: 'linha' })
    const root = s.nodes.find((n) => n.parent === null)!
    const mains = s.nodes.filter((n) => n.parent === root.id)
    const mid = root.y + root.height / 2
    let last = root.x + root.width
    mains.forEach((m, i) => {
      expect(Math.abs(m.y + m.height / 2 - mid)).toBeLessThan(1e-6)
      expect(m.x).toBeGreaterThanOrEqual(last)
      last = m.x + m.width
      for (const c of s.nodes.filter((n) => n.parent === m.id)) {
        if (i % 2 === 0) expect(c.y + c.height).toBeLessThanOrEqual(m.y)
        else expect(c.y).toBeGreaterThanOrEqual(m.y + m.height)
      }
    })
    for (const [i, a] of s.nodes.entries())
      for (const b of s.nodes.slice(i + 1))
        expect(a.x < b.x + b.width && b.x < a.x + a.width && a.y < b.y + b.height && b.y < a.y + a.height, `${a.text} × ${b.text}`).toBe(false)
  })
})

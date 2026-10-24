// Backend simulado para o modo preview.
//
// O app real fala com o Rust por comandos Tauri. No navegador não há Rust, e
// `invoke` lança. Este módulo implementa os mesmos comandos sobre um mapa em
// memória, com a mesma forma de snapshot, para que a interface inteira possa
// ser vista e ajustada sem compilar o aplicativo.
//
// Ativado por `?preview` na URL, ou automaticamente quando não há host Tauri e
// o Vite está em modo de desenvolvimento.

import type { CardFields, CardView, Edit, NodeSize, NoteCard, Snapshot, TabInfo } from './api'
import { docToMarkdown, markdownToDoc } from './notedoc'
import { rememberPreviewUrl } from './api'
import { coverFromMarkdown, hashTags, i18n, previewFromMarkdown, t, type Attachment } from '@wylho/atrio'

// ── modelo em memória ────────────────────────────────────────────────────

interface PNode {
  id: number
  text: string
  note: string
  side: 'left' | 'right'
  color: string | null
  colorToken: string | null
  folded: boolean
  card?: CardFields
  parent: number | null
  children: number[]
  x: number
  y: number
  width: number
  height: number
  /** Célula nas estruturas de tabela (layout.rs: `Node::cell`). */
  cell?: [number, number, number, number]
}

const CHAR_W = 7.6
const MIN_W = 84
const MIN_H = 44
const H_GAP = 68
const V_GAP = 16

let nodes = new Map<number, PNode>()
let nextId = 1
let rootId = 0
let dirty = false
let path: string | null = null
/** Tema do mapa, como `theme.style` no núcleo. */
let style: string | null = null
const MAP_STYLES = ['classico', 'contorno', 'sublinhado', 'vibrante', 'pilulas', 'mono']
/** Estrutura e espaçamento, como `theme.structure` e `theme.spacing` no núcleo. */
let structure: string | null = null
let spacing: string | null = null
const STRUCTURES = ['mapa', 'logico', 'organograma', 'arvore', 'matriz', 'tabela', 'linha']
const SPACINGS = ['compacto', 'normal', 'largo']
// Notas soltas, como no núcleo (`MindMap.notes`).
/** As cores de nota do Atrio (`nodo_core::model::NOTE_COLORS`). */
const NOTE_IDS = ['sand', 'sage', 'sky', 'rose', 'lilac', 'butter']

let freeNotes: NoteCard[] = []
const past: string[] = []
const future: string[] = []

function makeNode(text: string, side: 'left' | 'right' = 'right', note = '', color: string | null = null): PNode {
  const id = nextId++
  return {
    id,
    text,
    note,
    side,
    color,
    colorToken: color,
    folded: false,
    parent: null,
    children: [],
    x: 0,
    y: 0,
    width: Math.max(MIN_W, text.length * CHAR_W + 36),
    height: MIN_H,
  }
}

function snapshotState() {
  return JSON.stringify({
    nodes: [...nodes.entries()],
    nextId,
    rootId,
    dirty,
    path,
    style,
    structure,
    spacing,
    freeNotes,
  })
}

function restore(state: string) {
  const o = JSON.parse(state) as {
    nodes: [number, PNode][]
    nextId: number
    rootId: number
    dirty: boolean
    path: string | null
    style: string | null
    structure: string | null
    spacing: string | null
    freeNotes?: NoteCard[]
  }
  nodes = new Map(o.nodes)
  nextId = o.nextId
  rootId = o.rootId
  dirty = o.dirty
  path = o.path
  style = o.style
  structure = o.structure ?? null
  spacing = o.spacing ?? null
  freeNotes = o.freeNotes ?? []
}

// ── abas: cada uma guarda o mapa e o desfazer dela, como as abas do núcleo (`tabs.rs`) ──

interface PTab {
  id: number
  state: string
  past: string[]
  future: string[]
}

let tabs: PTab[] = []
let activeTab: number | null = null
let nextTab = 1

/** Guarda o mapa da frente na aba dele. */
function saveActive() {
  const t = tabs.find((x) => x.id === activeTab)
  if (t) Object.assign(t, { state: snapshotState(), past: [...past], future: [...future] })
}

function activate(t: PTab) {
  restore(t.state)
  past.splice(0, past.length, ...t.past)
  future.splice(0, future.length, ...t.future)
  activeTab = t.id
}

/** Uma aba nova com o mapa que está carregado agora, na frente. */
function pushTab() {
  const t: PTab = { id: nextTab++, state: '', past: [], future: [] }
  tabs.push(t)
  activeTab = t.id
  saveActive()
}

/** Sem nada feito, sem arquivo: abrir um arquivo ocupa o lugar dele. */
const untouched = () => path === null && !dirty && past.length === 0

function tabList(): TabInfo[] {
  return tabs.map((tab) => {
    if (tab.id === activeTab) return { id: tab.id, title: nodes.get(rootId)?.text ?? t('Sem título'), path, dirty }
    const o = JSON.parse(tab.state) as { nodes: [number, PNode][]; rootId: number; dirty: boolean; path: string | null }
    const root = o.nodes.find(([id]) => id === o.rootId)?.[1]
    return { id: tab.id, title: root?.text ?? t('Sem título'), path: o.path, dirty: o.dirty }
  })
}

function snapshotOf(): Snapshot {
  return { ...mapSnapshot(), tabs: tabList(), active: activeTab }
}

function mapSnapshot(): Omit<Snapshot, 'tabs' | 'active'> {
  // Como no núcleo: os descendentes de um nó recolhido não aparecem.
  const list: PNode[] = []
  const visit = (id: number) => {
    const n = nodes.get(id)
    if (!n) return
    list.push(n)
    if (!n.folded) n.children.forEach(visit)
  }
  visit(rootId)
  if (list.length === 0) {
    return {
      nodes: [],
      connections: [],
      notes: [],
      dirty,
      path,
      title: t('Sem título'),
      can_undo: past.length > 0,
      can_redo: future.length > 0,
      stale: false,
      style,
      structure,
      spacing,
      bounds: { min_x: 0, min_y: 0, max_x: 0, max_y: 0 },
    }
  }

  let minX = Infinity
  let minY = Infinity
  let maxX = -Infinity
  let maxY = -Infinity

  const flat = list.map((n) => {
    // Na tabela, a célula inteira entra na extensão (como o núcleo).
    const [bx, by, bw, bh] = n.cell ?? [n.x, n.y, n.width, n.height]
    minX = Math.min(minX, bx)
    minY = Math.min(minY, by)
    maxX = Math.max(maxX, bx + bw)
    maxY = Math.max(maxY, by + bh)
    return {
      id: n.id,
      text: n.text,
      note: n.note,
      side: n.side as 'left' | 'right',
      depth: depthOf(n.id),
      x: n.x,
      y: n.y,
      width: n.width,
      height: n.height,
      color: n.colorToken,
      folded: n.folded,
      card: n.card ?? {},
      view: viewOf(n.note),
      child_ids: [...n.children],
      parent: n.parent,
      has_children: n.children.length > 0,
      hidden: n.folded ? descendants(n.id) : 0,
      cell: n.cell ? ([...n.cell] as [number, number, number, number]) : undefined,
    }
  })

  for (const f of freeNotes) {
    minX = Math.min(minX, f.x)
    minY = Math.min(minY, f.y)
    maxX = Math.max(maxX, f.x + f.width)
    maxY = Math.max(maxY, f.y + f.height)
  }

  // Pré-ordem: o inspetor e o desenho esperam a raiz primeiro.
  flat.sort((a, b) => a.depth - b.depth || a.id - b.id)

  return {
    nodes: flat,
    connections: [],
    notes: freeNotes.map((f) => ({ ...f, card: { ...f.card }, view: viewOf(f.text) })),
    dirty,
    path,
    title: nodes.get(rootId)?.text ?? t('Sem título'),
    can_undo: past.length > 0,
    can_redo: future.length > 0,
    stale: false,
    style,
    structure,
    spacing,
    bounds: { min_x: minX, min_y: minY, max_x: maxX, max_y: maxY },
  }
}

function depthOf(id: number): number {
  let d = 0
  let cur = nodes.get(id)?.parent
  while (cur !== null && cur !== undefined) {
    d++
    cur = nodes.get(cur)?.parent
  }
  return d
}

// ── layout: espelha nodo-core/src/layout.rs ────────────────────────────

function measure(n: PNode) {
  const lines = n.text.split('\n')
  const longest = Math.max(...lines.map((l) => l.length), 1)
  n.width = Math.max(MIN_W, longest * CHAR_W + 36)
  n.height = Math.max(MIN_H, lines.length * 19 + 22)
}

/** Modo e folgas da passada de layout (layout.rs: `Params`). */
function params() {
  const mode =
    structure === 'logico' ? 'right'
    : structure === 'organograma' ? 'down'
    : structure === 'arvore' ? 'tree'
    : structure === 'matriz' ? 'matrix'
    : structure === 'tabela' ? 'treetable'
    : structure === 'linha' ? 'timeline'
    : 'map'
  const k = spacing === 'compacto' ? 0.6 : spacing === 'largo' ? 1.6 : 1
  return { mode, hGap: H_GAP * k, vGap: V_GAP * k }
}

const before = (c: PNode) => params().mode === 'map' && c.side === 'left'

/** Quanto uma sub-árvore ocupa de través, para irmãos nunca se sobreporem (layout.rs: `extent`). */
function extent(n: PNode): number {
  const { mode, vGap } = params()
  const own = mode === 'down' ? n.width : n.height
  if (n.children.length === 0 || n.folded) return own
  const band = (ids: number[]) =>
    ids.length === 0 ? 0 : ids.reduce((a, id) => a + extent(nodes.get(id)!), 0) + vGap * (ids.length - 1)
  const kids = n.children.filter((id) => nodes.has(id))
  const pre = kids.filter((id) => before(nodes.get(id)!))
  const post = kids.filter((id) => !before(nodes.get(id)!))
  return Math.max(own, band(pre), band(post))
}

/** Posiciona `n` numa faixa que começa em `top`, com a borda próxima em `x` (layout.rs: `place`). */
function place(n: PNode, x: number, top: number) {
  const { mode, hGap, vGap } = params()
  const ext = extent(n)
  if (mode === 'down') {
    n.x = top + (ext - n.width) / 2
    n.y = x
  } else {
    n.x = x
    n.y = top + (ext - n.height) / 2
  }
  if (n.children.length === 0 || n.folded) return
  for (const pre of [true, false]) {
    const group = n.children.map((id) => nodes.get(id)!).filter((c) => c && before(c) === pre)
    if (group.length === 0) continue
    const total = group.reduce((a, c) => a + extent(c), 0) + vGap * (group.length - 1)
    let cursor = top + (ext - total) / 2
    for (const c of group) {
      const near = mode === 'down' ? n.y + n.height + hGap : pre ? n.x - hGap - c.width : n.x + n.width + hGap
      place(c, near, cursor)
      cursor += extent(c) + vGap
    }
  }
}

/** Lista recuada (layout.rs: `place_tree`). Devolve onde começa a próxima linha. */
function placeTree(n: PNode, x: number, y: number): number {
  const { hGap, vGap } = params()
  n.x = x
  n.y = y
  let next = y + n.height + vGap * 0.5
  if (n.folded) return next
  const indent = Math.max(18, hGap * 0.6)
  for (const id of n.children) {
    const c = nodes.get(id)
    if (c) next = placeTree(c, x + indent, next)
  }
  return next
}

/** Folga do tópico dentro da célula (layout.rs: `pad`). */
const cellPad = () => Math.max(6, params().vGap * 0.5)
const kids = (n: PNode) => (n.folded ? [] : n.children.map((id) => nodes.get(id)!).filter(Boolean))

/** Tabela em árvore, da esquerda para a direita (layout.rs: `place_tree_table`). */
function placeTreeTable(root: PNode) {
  const pad = cellPad()
  const cols: number[] = []
  const measureCols = (n: PNode, d: number) => {
    cols[d] = Math.max(cols[d] ?? 0, n.width + pad * 2)
    for (const c of kids(n)) measureCols(c, d + 1)
  }
  const span = (n: PNode): number => {
    const own = n.height + pad * 2
    const cs = kids(n)
    return cs.length ? Math.max(own, cs.reduce((a, c) => a + span(c), 0)) : own
  }
  const put = (n: PNode, x: number, y: number, h: number, d: number) => {
    const cs = kids(n)
    const w = cs.length ? cols[d] : cols.slice(d).reduce((a, b) => a + b, 0)
    n.cell = [x, y, w, h]
    n.x = d === 0 ? x + (w - n.width) / 2 : x + pad
    n.y = y + (h - n.height) / 2
    if (!cs.length) return
    const spans = cs.map(span)
    const extra = Math.max(0, h - spans.reduce((a, b) => a + b, 0)) / spans.length
    let cy = y
    cs.forEach((c, i) => {
      put(c, x + cols[d], cy, spans[i] + extra, d + 1)
      cy += spans[i] + extra
    })
  }
  measureCols(root, 0)
  put(root, 0, 0, span(root), 0)
}

/** Linha do tempo (layout.rs: `place_timeline`): principais em sequência, galhos alternando acima e abaixo. */
function placeTimeline(root: PNode) {
  const { hGap, vGap } = params()
  root.x = 0
  root.y = -root.height / 2
  const mains = kids(root)
  const indent = Math.max(16, hGap * 0.4)
  const band = Math.max(0, ...mains.map((m) => m.height)) / 2
  const gap = vGap * 1.5
  // O galho cresce para a direita, como no lógico.
  const ext = (n: PNode): number => {
    const cs = kids(n)
    return cs.length ? Math.max(n.height, cs.reduce((a, c) => a + ext(c), 0) + vGap * (cs.length - 1)) : n.height
  }
  const putRight = (n: PNode, x: number, top: number) => {
    const e = ext(n)
    n.x = x
    n.y = top + (e - n.height) / 2
    const cs = kids(n)
    const total = cs.reduce((a, c) => a + ext(c), 0) + vGap * (cs.length - 1)
    let cursor = top + (e - total) / 2
    for (const c of cs) {
      putRight(c, n.x + n.width + hGap, cursor)
      cursor += ext(c) + vGap
    }
  }
  const rightEdge = (n: PNode): number => Math.max(n.x + n.width, ...kids(n).map(rightEdge))
  let x = root.width + hGap
  const edge = [-Infinity, -Infinity]
  mains.forEach((m, i) => {
    const side = i % 2
    x = Math.max(x, edge[side] + hGap - indent)
    m.x = x
    m.y = -m.height / 2
    let reach = x + m.width
    const cs = kids(m)
    if (cs.length) {
      const total = cs.reduce((a, c) => a + ext(c), 0) + vGap * (cs.length - 1)
      let cursor = side === 0 ? -band - gap - total : band + gap
      for (const c of cs) {
        putRight(c, x + indent, cursor)
        cursor += ext(c) + vGap
        reach = Math.max(reach, rightEdge(c))
      }
      edge[side] = Math.max(edge[side], reach)
    }
    x = x + m.width + hGap
  })
}

/** Matriz (layout.rs: `place_matrix`). */
function placeMatrix(root: PNode) {
  const { hGap, vGap } = params()
  const pad = cellPad()
  const indent = Math.max(14, hGap * 0.4)
  const gap = Math.max(4, vGap * 0.4)
  const content = (n: PNode): [number, number] => {
    let [w, h] = [n.width, n.height]
    for (const c of kids(n)) {
      const [cw, ch] = content(c)
      w = Math.max(w, cw + indent)
      h += gap + ch
    }
    return [w, h]
  }
  const putList = (n: PNode, x: number, y: number): number => {
    n.x = x
    n.y = y
    let next = y + n.height
    for (const c of kids(n)) next = putList(c, x + indent, next + gap)
    return next
  }
  const rowsN = kids(root)
  const cols = Math.max(0, ...rowsN.map((r) => kids(r).length))
  const headW = Math.max(0, ...rowsN.map((r) => r.width)) + pad * 2
  const colW: number[] = Array(cols).fill(0)
  const rowH = rowsN.map((r) => {
    let h = r.height
    kids(r).forEach((c, j) => {
      const [cw, ch] = content(c)
      colW[j] = Math.max(colW[j], cw + pad * 2)
      h = Math.max(h, ch)
    })
    return h + pad * 2
  })
  const tableW = Math.max(headW + colW.reduce((a, b) => a + b, 0), root.width + pad * 2)
  if (cols) colW[cols - 1] += tableW - headW - colW.reduce((a, b) => a + b, 0)
  const titleH = root.height + pad * 2
  root.cell = [0, 0, tableW, titleH]
  root.x = pad
  root.y = pad
  let y = titleH
  rowsN.forEach((r, i) => {
    const h = rowH[i]
    r.cell = [0, y, cols ? headW : tableW, h]
    r.x = pad
    r.y = y + pad
    let x = headW
    kids(r).forEach((c, j) => {
      putList(c, x + pad, y + pad)
      c.cell = [x, y, colW[j], h]
      x += colW[j]
    })
    y += h
  })
}

/** Mede tudo pela estimativa e posiciona (layout.rs: `layout`). */
function relayout() {
  for (const n of nodes.values()) measure(n)
  reposition()
}

/** Só posiciona, com os tamanhos que já estão nos nós (layout.rs: `reposition`). */
function reposition() {
  const root = nodes.get(rootId)
  if (!root) return
  for (const n of nodes.values()) n.cell = undefined
  const mode = params().mode
  if (mode === 'tree') placeTree(root, 0, 0)
  else if (mode === 'matrix') placeMatrix(root)
  else if (mode === 'treetable') placeTreeTable(root)
  else if (mode === 'timeline') placeTimeline(root)
  else place(root, 0, -extent(root) / 2)
}

// ── histórico ────────────────────────────────────────────────────────────

/** Como `layout::clear_notes` do núcleo: nota solta que ficou em cima de um tópico vai para a direita do mapa. */
function clearNotes() {
  const GAP = 32
  const boxes: [number, number, number, number][] = []
  const walk = (id: number) => {
    const n = nodes.get(id)
    if (!n) return
    boxes.push(n.cell ?? [n.x, n.y, n.width, n.height])
    if (!n.folded) n.children.forEach(walk)
  }
  walk(rootId)
  const right = Math.max(...boxes.map((b) => b[0] + b[2])) + GAP * 2
  let nextY = Math.min(...boxes.map((b) => b[1]))
  const hits = (r: [number, number, number, number]) =>
    boxes.some((b) => r[0] < b[0] + b[2] && b[0] < r[0] + r[2] && r[1] < b[1] + b[3] && b[1] < r[1] + r[3])
  for (const f of freeNotes) {
    const r: [number, number, number, number] = [f.x, f.y, f.width, f.height]
    if (!hits(r)) {
      boxes.push(r)
      continue
    }
    for (;;) {
      const at: [number, number, number, number] = [right, nextY, f.width, f.height]
      nextY += Math.max(f.height, 40) + GAP
      if (!hits(at)) {
        f.x = at[0]
        f.y = at[1]
        boxes.push(at)
        break
      }
    }
  }
}

function commit(fn: () => void, relayoutAfter = true) {
  past.push(snapshotState())
  if (past.length > 200) past.shift()
  future.length = 0
  fn()
  // Como o Editor do núcleo: posiciona sem medir de novo; quem mediu foi a tela.
  if (relayoutAfter) reposition()
  dirty = true
}

// ── dados iniciais: um mapa de exemplo, para o preview ter o que mostrar ──

type Branch = [text: string, children: string[]]

/**
 * O mapa de exemplo, nos dois idiomas (o da interface na hora de começar a prévia): a mesma estrutura, os mesmos
 * cards e a mesma nota solta. Tudo inventado (o repositório é público). Os ramos vão na ordem de `fillDemo`: o segundo
 * filho do Design ganha `designSystem`, e os três últimos ramos ficam à esquerda.
 */
export const DEMO = {
  pt: {
    file: 'exemplo.nodo',
    root: 'Lançamento do app',
    rootNote: 'Plano do trimestre. **Responsável:** time de produto.',
    branches: [
      ['Descoberta', ['Entrevistas', 'Análise de mercado']],
      ['Design', ['Wireframes', 'Design system']],
      ['Construção', ['Núcleo', 'Layout', 'Servidor MCP', 'Janela']],
      ['Lançamento', ['Beta', 'Documentação', 'Versão 1.0']],
      ['Divulgação', ['Site', 'Vídeo curto', 'Comunidade']],
      ['Pesquisa', ['Concorrentes', 'Artigos']],
    ] as Branch[],
    designSystem: ['Cores', 'Componentes'],
    mcpNote: 'Dezessete ferramentas, o mesmo desfazer da janela.',
    discoveryNote: '- 12 entrevistas com quem já usa mapas\n- Dor nº 1: **exportar para slides**',
    buildNote: '- [x] Núcleo\n- [x] Layout\n- [ ] Servidor MCP\n- [ ] Janela',
    researchTag: 'pesquisa',
    betaNote: 'Convidar 30 pessoas da lista de espera. #convites',
    freeTitle: 'Ideias soltas',
    freeText: '- [ ] Pedir feedback da beta\n- [ ] Ver preço do domínio\n\nAnotar tudo antes da reunião. #reuniao',
  },
  en: {
    file: 'example.nodo',
    root: 'App launch',
    rootNote: 'Plan for the quarter. **Owner:** product team.',
    branches: [
      ['Discovery', ['Interviews', 'Market analysis']],
      ['Design', ['Wireframes', 'Design system']],
      ['Build', ['Core', 'Layout', 'MCP server', 'Window']],
      ['Launch', ['Beta', 'Documentation', 'Version 1.0']],
      ['Outreach', ['Website', 'Short video', 'Community']],
      ['Research', ['Competitors', 'Articles']],
    ] as Branch[],
    designSystem: ['Colors', 'Components'],
    mcpNote: 'Seventeen tools, the same undo as the window.',
    discoveryNote: '- 12 interviews with people who already use mind maps\n- Top pain point: **exporting to slides**',
    buildNote: '- [x] Core\n- [x] Layout\n- [ ] MCP server\n- [ ] Window',
    researchTag: 'research',
    betaNote: 'Invite 30 people from the waiting list. #invites',
    freeTitle: 'Loose ideas',
    freeText: '- [ ] Ask for beta feedback\n- [ ] Check the domain price\n\nWrite everything down before the meeting. #meeting',
  },
}

/** Começa o preview: uma aba só, com o mapa de exemplo (no idioma da interface). */
export function seedDemo() {
  tabs = []
  activeTab = null
  fillDemo()
  pushTab()
}

function fillDemo() {
  nodes.clear()
  nextId = 1
  past.length = 0
  future.length = 0
  dirty = false
  const x = DEMO[i18n.locale === 'en' ? 'en' : 'pt']
  path = x.file
  style = null
  freeNotes = []

  // Um mapa inventado (o repositório é público): o lançamento de um app fictício, com ramos dos dois lados.
  const root = makeNode(x.root, 'right', x.rootNote)
  rootId = root.id
  root.parent = null
  nodes.set(root.id, root)

  const branch = (parent: PNode, text: string, children: string[] = [], side = parent.side) => {
    // Sem cor fixa: quem pinta os ramos é o tema do mapa (a cor escolhida à mão continua valendo por cima).
    const n = makeNode(text, side, '', null)
    n.parent = parent.id
    parent.children.push(n.id)
    nodes.set(n.id, n)
    for (const c of children) branch(n, c)
    return n
  }

  const [discovery, design, build, launch, outreach, research] = x.branches
  const found = branch(root, ...discovery)
  const ds = nodes.get(branch(root, ...design).children[1])!
  for (const c of x.designSystem) branch(ds, c)
  const core = branch(root, ...build)
  const ship = branch(root, ...launch, 'left')
  branch(root, ...outreach, 'left')
  const last = branch(root, ...research, 'left')

  // Notas (os nós com nota viram card) e um ramo recolhido, para os marcadores aparecerem.
  nodes.get(core.children[2])!.note = x.mcpNote
  found.note = x.discoveryNote
  core.note = x.buildNote
  core.card = { tags: ['mvp'], emoji: '🛠️', status_on: true, reminder_doing: true }
  found.card = { emoji: '🔎', tags: [x.researchTag] }
  const beta = nodes.get(ship.children[0])!
  beta.note = x.betaNote
  beta.card = { reminder_at: Date.now() + 2 * 86_400_000, tags: ['beta'] }
  last.folded = true

  relayout()
  // Uma nota solta (o card padrão, sem ramo), no vão embaixo da raiz.
  const r = nodes.get(root.id)!
  freeNotes = [
    {
      id: 1,
      // No meio do vão entre os ramos da esquerda e o card da Construção (a largura do vão muda com o idioma).
      x: Math.round((Math.max(...[ship, last].map((n) => n.x + n.width)) + core.x - 240) / 2),
      y: Math.round(r.y + r.height + 150),
      width: 240,
      height: 150,
      title: x.freeTitle,
      text: x.freeText,
      color: 'lilac',
      card: { emoji: '💡' },
    },
  ]
}

// ── comandos ─────────────────────────────────────────────────────────────

export const previewBackend = {
  get_snapshot: () => snapshotOf(),

  new_map: ({ title }: { title: string }) => {
    saveActive()
    nodes.clear()
    nextId = 1
    past.length = 0
    future.length = 0
    freeNotes = []
    const root = makeNode(title || t('Tópico central'))
    rootId = root.id
    nodes.set(root.id, root)
    dirty = false
    path = null
    style = null
    relayout()
    pushTab()
    return snapshotOf()
  },

  open_map: ({ path: p }: { path: string }) => {
    saveActive()
    const open = tabs.find((t) => (t.id === activeTab ? path : (JSON.parse(t.state) as { path: string | null }).path) === p)
    if (open) {
      activate(open)
      return snapshotOf()
    }
    const reuse = activeTab !== null && untouched()
    fillDemo()
    path = p
    if (reuse) saveActive()
    else pushTab()
    return snapshotOf()
  },

  switch_tab: ({ id }: { id: number }) => {
    const t = tabs.find((x) => x.id === id)
    if (!t) throw new Error('essa aba não está mais aberta')
    saveActive()
    activate(t)
    return snapshotOf()
  },

  close_tab: ({ id }: { id: number }) => {
    saveActive()
    const i = tabs.findIndex((x) => x.id === id)
    if (i >= 0) {
      const wasActive = id === activeTab
      tabs.splice(i, 1)
      if (wasActive) {
        const next = tabs[i] ?? tabs[i - 1]
        if (next) activate(next)
        else {
          activeTab = null
          nodes.clear()
          freeNotes = []
          past.length = 0
          future.length = 0
          path = null
          dirty = false
        }
      }
    }
    return snapshotOf()
  },

  save_map: ({ path: p }: { path?: string | null }) => {
    if (p) path = p
    dirty = false
    return snapshotOf()
  },

  add_node: ({ parent, text }: { parent: number; text: string }) => {
    const parentNode = nodes.get(parent)
    if (!parentNode) throw new Error(`node ${parent} not found`)
    let newId = 0
    commit(() => {
      const n = makeNode(text)
      n.parent = parent
      n.side = parentNode.side
      nodes.set(n.id, n)
      parentNode.children.push(n.id)
      newId = n.id
    })
    return snapshotOf()
  },

  add_topic: ({
    parent,
    text,
    note,
    color = null,
    card = {},
  }: {
    parent: number
    text: string
    note: string
    color?: string | null
    card?: CardFields
  }) => {
    const parentNode = nodes.get(parent)
    if (!parentNode) throw new Error(`node ${parent} not found`)
    commit(() => {
      const n = makeNode(text, parentNode.side, note, color)
      n.card = cleanCard(card)
      n.parent = parent
      nodes.set(n.id, n)
      parentNode.children.push(n.id)
    })
    return snapshotOf()
  },

  remove_node: ({ id }: { id: number }) => {
    if (id === rootId) throw new Error('cannot remove the root node')
    commit(() => {
      const n = nodes.get(id)
      if (!n) throw new Error(`node ${id} not found`)
      const parent = n.parent !== null ? nodes.get(n.parent) : null
      if (parent) parent.children = parent.children.filter((c) => c !== id)
      const stack = [id]
      while (stack.length) {
        const cur = stack.pop()!
        for (const c of nodes.get(cur)?.children ?? []) stack.push(c)
        nodes.delete(cur)
      }
    })
    return snapshotOf()
  },

  set_text: ({ id, text }: { id: number; text: string }) => {
    commit(() => {
      const n = nodes.get(id)
      if (!n) throw new Error(`node ${id} not found`)
      n.text = text
      measure(n)
    })
    return snapshotOf()
  },

  set_note: ({ id, note }: { id: number; note: string }) => {
    commit(() => {
      const n = nodes.get(id)
      if (!n) throw new Error(`node ${id} not found`)
      n.note = note
    })
    return snapshotOf()
  },

  set_card: ({ id, card }: { id: number; card: CardFields }) => {
    commit(() => {
      const n = nodes.get(id)
      if (!n) throw new Error(`node ${id} not found`)
      n.card = cleanCard(card)
    }, false)
    return snapshotOf()
  },

  note_doc: ({ id }: { id: number }) => markdownToDoc(nodes.get(id)?.note ?? ''),

  markdown_doc: ({ markdown }: { markdown: string }) => markdownToDoc(markdown),

  set_note_doc: ({ id, doc }: { id: number; doc: unknown }) => {
    const note = docToMarkdown(doc, (h) => previewFiles.get(h)?.name)
    commit(() => {
      const n = nodes.get(id)
      if (!n) throw new Error(`node ${id} not found`)
      n.note = note
    })
    return snapshotOf()
  },

  add_free_note: ({ x, y, text }: { x: number; y: number; text: string }) => {
    commit(() => {
      const id = freeNotes.reduce((m, f) => Math.max(m, f.id), 0) + 1
      freeNotes.push({ id, x, y, width: 240, height: 120, title: '', text, color: null, card: {} })
    }, false)
    return snapshotOf()
  },

  set_free_note: ({ id, text }: { id: number; text: string }) => {
    commit(() => {
      const f = freeNotes.find((n) => n.id === id)
      if (!f) throw new Error(`note ${id} not found`)
      f.text = text
    }, false)
    return snapshotOf()
  },

  place_free_note: ({ id, x, y, width, height }: { id: number; x: number; y: number; width: number; height: number }) => {
    commit(() => {
      const f = freeNotes.find((n) => n.id === id)
      if (!f) throw new Error(`note ${id} not found`)
      Object.assign(f, { x, y, width: Math.min(1600, Math.max(120, width)), height: Math.min(1600, Math.max(80, height)) })
    }, false)
    return snapshotOf()
  },

  free_note_doc: ({ id }: { id: number }) => markdownToDoc(freeNotes.find((n) => n.id === id)?.text ?? ''),

  set_free_note_doc: ({ id, doc }: { id: number; doc: unknown }) => {
    const text = docToMarkdown(doc, (h) => previewFiles.get(h)?.name)
    commit(() => {
      const f = freeNotes.find((n) => n.id === id)
      if (!f) throw new Error(`note ${id} not found`)
      f.text = text
    }, false)
    return snapshotOf()
  },

  set_free_card: ({ id, title, color, card }: { id: number; title: string; color: string | null; card: CardFields }) => {
    commit(() => {
      const f = freeNotes.find((n) => n.id === id)
      if (!f) throw new Error(`note ${id} not found`)
      Object.assign(f, { title: title.trim(), color: color && NOTE_IDS.includes(color) ? color : null, card: cleanCard(card) })
    }, false)
    return snapshotOf()
  },

  remove_free_note: ({ id }: { id: number }) => {
    commit(() => {
      if (!freeNotes.some((n) => n.id === id)) throw new Error(`note ${id} not found`)
      freeNotes = freeNotes.filter((n) => n.id !== id)
    }, false)
    return snapshotOf()
  },

  set_color: ({ id, color }: { id: number; color: string | null }) => {
    commit(() => {
      const n = nodes.get(id)
      if (!n) throw new Error(`node ${id} not found`)
      n.colorToken = color
      n.color = color
    })
    return snapshotOf()
  },

  set_side: ({ id, side }: { id: number; side: 'left' | 'right' }) => {
    commit(() => {
      const stack = [id]
      while (stack.length) {
        const n = nodes.get(stack.pop()!)
        if (!n) continue
        n.side = side
        stack.push(...n.children)
      }
    })
    return snapshotOf()
  },

  set_map_style: ({ style: next }: { style: string | null }) => {
    if (next !== null && !MAP_STYLES.includes(next)) throw new Error(`unknown map theme "${next}"`)
    commit(() => {
      style = next
    })
    return snapshotOf()
  },

  set_structure: ({ structure: next }: { structure: string | null }) => {
    if (next !== null && !STRUCTURES.includes(next)) throw new Error(`unknown map structure "${next}"`)
    commit(() => {
      structure = next
    })
    clearNotes()
    return snapshotOf()
  },

  set_spacing: ({ spacing: next }: { spacing: string | null }) => {
    if (next !== null && !SPACINGS.includes(next)) throw new Error(`unknown spacing "${next}"`)
    commit(() => {
      spacing = next
    })
    clearNotes()
    return snapshotOf()
  },

  relayout: () => {
    commit(() => {})
    clearNotes()
    return snapshotOf()
  },

  // Espelha o `apply_edits` do app: as edições de um grupo como um passo só, tudo ou nada; tópico dentro de outro do
  // grupo vai com ele (só o de cima é apagado ou movido).
  apply_edits: ({ edits }: { edits: Edit[] }) => {
    const inside = (id: number, set: Set<number>) => {
      for (let p = nodes.get(id)?.parent ?? null; p !== null; p = nodes.get(p)?.parent ?? null) if (set.has(p)) return true
      return false
    }
    const removed = new Set(edits.flatMap((e) => (e.kind === 'remove' ? [e.id] : [])))
    const moved = new Set(edits.flatMap((e) => (e.kind === 'move' ? [e.id] : [])))
    const before = snapshotState()
    const depth = past.length
    try {
      for (const e of edits) {
        if (e.kind === 'remove') {
          if (e.id !== rootId && !inside(e.id, removed)) previewBackend.remove_node({ id: e.id })
        } else if (e.kind === 'color') previewBackend.set_color({ id: e.id, color: e.color })
        else if (e.kind === 'card') previewBackend.set_card({ id: e.id, card: e.card })
        else if (e.kind === 'move') {
          if (!inside(e.id, moved)) previewBackend.move_node({ id: e.id, x: e.x, y: e.y })
        } else if (e.kind === 'topic') previewBackend.add_topic(e)
        else if (e.kind === 'placeFree') previewBackend.place_free_note(e)
        else if (e.kind === 'removeFree') previewBackend.remove_free_note({ id: e.id })
        else if (e.kind === 'relayout') previewBackend.relayout()
      }
    } catch (err) {
      restore(before)
      past.length = depth
      throw err
    }
    past.length = depth
    if (edits.length) past.push(before)
    return snapshotOf()
  },

  // Espelha nodo_core::export (Markdown e OPML, os formatos de texto que a folha Exportar oferece).
  export_text: ({ format }: { format: string }) => {
    const root = nodes.get(rootId)
    if (!root) return ''
    const line = (t: string) => t.split(/\s+/).filter(Boolean).join(' ')
    const esc = (t: string) => t.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;').replace(/"/g, '&quot;')
    const kids = (n: PNode) => n.children.map((id) => nodes.get(id)!).filter(Boolean)
    if (format === 'markdown') {
      let out = `# ${line(root.text)}\n\n${root.note.trim() ? root.note.trim() + '\n\n' : ''}`
      const walk = (n: PNode, d: number) => {
        out += `${'  '.repeat(d)}- ${line(n.text)}\n`
        for (const l of n.note.trim() ? n.note.trim().split('\n') : []) out += `${'  '.repeat(d)}  ${l}\n`
        kids(n).forEach((c) => walk(c, d + 1))
      }
      kids(root).forEach((c) => walk(c, 0))
      return out
    }
    if (format === 'opml') {
      let out = `<?xml version="1.0" encoding="UTF-8"?>\n<opml version="2.0">\n  <head>\n    <title>${esc(line(root.text))}</title>\n  </head>\n  <body>\n`
      const walk = (n: PNode, d: number) => {
        const pad = '  '.repeat(d + 2)
        const note = n.note.trim() ? ` _note="${esc(n.note.trim())}"` : ''
        if (n.children.length === 0) out += `${pad}<outline text="${esc(line(n.text))}"${note}/>\n`
        else {
          out += `${pad}<outline text="${esc(line(n.text))}"${note}>\n`
          kids(n).forEach((c) => walk(c, d + 1))
          out += `${pad}</outline>\n`
        }
      }
      walk(root, 0)
      return out + '  </body>\n</opml>\n'
    }
    throw new Error(`unknown export format "${format}"`)
  },

  reload_map: () => snapshotOf(),

  quit_app: () => undefined,

  toggle_fold: ({ id }: { id: number }) => {
    commit(() => {
      const n = nodes.get(id)
      if (!n) throw new Error(`node ${id} not found`)
      n.folded = !n.folded
    })
    return snapshotOf()
  },

  reorder_node: ({ id, delta }: { id: number; delta: number }) => {
    const parent = [...nodes.values()].find((n) => n.children.includes(id))
    const me = nodes.get(id)
    if (!parent || !me) throw new Error('there is no sibling in that direction to swap with')
    const at = parent.children.indexOf(id)
    const fits = (cid: number) => parent.parent !== null || nodes.get(cid)?.side === me.side
    let to = -1
    if (delta < 0) for (let i = at - 1; i >= 0 && to < 0; i--) if (fits(parent.children[i])) to = i
    if (delta > 0) for (let i = at + 1; i < parent.children.length && to < 0; i++) if (fits(parent.children[i])) to = i
    if (to < 0) throw new Error('there is no sibling in that direction to swap with')
    commit(() => {
      parent.children[at] = parent.children[to]
      parent.children[to] = id
    })
    return snapshotOf()
  },

  move_node: ({ id, x, y }: { id: number; x: number; y: number }) => {
    commit(
      () => {
        const n = nodes.get(id)
        if (!n) throw new Error(`node ${id} not found`)
        n.x = x
        n.y = y
      },
      false, // posição manual: o relayout a desfaria
    )
    return snapshotOf()
  },

  set_sizes: ({ sizes, notes }: { sizes: NodeSize[]; notes?: NodeSize[] | null }) => {
    for (const s of notes ?? []) {
      const f = freeNotes.find((x) => x.id === s.id)
      if (f) f.height = s.height
    }
    for (const s of sizes) {
      const n = nodes.get(s.id)
      if (n) {
        n.width = s.width
        n.height = s.height
      }
    }
    reposition() // como o núcleo: as medidas vieram da tela, não estimar de novo
    return snapshotOf()
  },

  export_layout: ({ sizes, unfold }: { sizes: NodeSize[]; unfold: boolean }) => {
    // Como o núcleo: arruma uma cópia por um instante e devolve tudo exatamente como estava.
    const saved = new Map([...nodes].map(([id, n]) => [id, { ...n, children: [...n.children] }]))
    if (unfold) for (const n of nodes.values()) n.folded = false
    const out = previewBackend.set_sizes({ sizes })
    nodes = saved
    return out
  },

  undo: () => {
    const prev = past.pop()
    if (prev) {
      future.push(snapshotState())
      restore(prev)
    }
    return snapshotOf()
  },

  redo: () => {
    const next = future.pop()
    if (next) {
      past.push(snapshotState())
      restore(next)
    }
    return snapshotOf()
  },

  search_nodes: ({ query, limit }: { query: string; limit?: number }) => {
    const q = query.trim().toLowerCase()
    if (!q) return []
    return [...nodes.values()]
      .filter((n) => n.text.toLowerCase().includes(q) || n.note.toLowerCase().includes(q))
      .slice(0, limit ?? 50)
      .map((n) => n.id)
  },

  mcp_info: () => ({
    command: '/Applications/Nodo.app/Contents/MacOS/nodo',
    args: ['--mcp'],
    claudeCode: 'claude mcp add --scope user nodo -- "/Applications/Nodo.app/Contents/MacOS/nodo" --mcp',
    desktopConfig: '~/Library/Application Support/Claude/claude_desktop_config.json',
    desktopFound: true,
    desktopInstalled: false,
  }),

  mcp_install: () => previewBackend.mcp_info(),
  mcp_uninstall: () => previewBackend.mcp_info(),

  // `?update` simula uma versão nova, para ver o aviso.
  update_supported: () => true,
  update_check: () => (new URLSearchParams(location.search).has('update') ? { version: '0.3.0', notes: null } : null),
  update_install: () => new Promise<void>(() => {}),

  log_measurements: () => undefined,
}

/** As regras do card no núcleo (`CardFields::cleaned`, com `atrio::text`). */
function cleanCard(c: CardFields): CardFields {
  const tags: string[] = []
  for (const t of c.tags ?? []) {
    const v = t.trim().replace(/^#+/, '').split(/\s+/).filter(Boolean).join('-').toLowerCase()
    if (v && !tags.includes(v)) tags.push(v)
  }
  const at = c.reminder_at ?? null
  const keep = at != null || !!c.status_on
  const out: CardFields = {}
  if (tags.length) out.tags = tags
  if (c.emoji?.trim()) out.emoji = c.emoji.trim()
  if (at != null) out.reminder_at = at
  if (at != null && ['day', 'week', 'month', 'year'].includes(c.reminder_repeat ?? '')) out.reminder_repeat = c.reminder_repeat
  if (keep && c.reminder_done) out.reminder_done = true
  if (keep && c.reminder_doing && !c.reminder_done) out.reminder_doing = true
  if (c.status_on) out.status_on = true
  return out
}

// ── anexos da prévia: em memória, com o endereço de um blob para mostrar ──

const previewFiles = new Map<string, Attachment>()

export async function previewImport(file: Blob, name: string): Promise<Attachment> {
  const bytes = new Uint8Array(await file.arrayBuffer())
  const digest = new Uint8Array(await crypto.subtle.digest('SHA-256', bytes))
  const hash = [...digest.slice(0, 16)].map((b) => b.toString(16).padStart(2, '0')).join('')
  const known = previewFiles.get(hash)
  if (known) return known
  const mime = file.type || 'application/octet-stream'
  const kind = mime.startsWith('image/') ? 'image' : mime.startsWith('audio/') ? 'audio' : mime.startsWith('video/') ? 'video'
    : mime === 'application/pdf' ? 'pdf' : 'other'
  const a = {
    hash, name, mime, kind, bytes: bytes.length, addedAt: Date.now(),
    origBytes: null, width: null, height: null, palette: null, tone: null,
  } as Attachment
  previewFiles.set(hash, a)
  rememberPreviewUrl(hash, URL.createObjectURL(new Blob([bytes], { type: mime })))
  return a
}

export function previewAttachments(hashes: string[]): Attachment[] {
  return hashes.flatMap((h) => previewFiles.get(h) ?? [])
}

/**
 * A prévia do card na prévia do navegador, sem o núcleo: a versão em TypeScript do Atrio. No app, ela vem pronta do
 * núcleo (`nodo_core::preview`, a regra em Rust, a mesma do Ideario).
 */
function viewOf(note: string): CardView | undefined {
  if (!note.trim()) return undefined
  const images = [...note.matchAll(/!\[[^\]]*\]\(ideario:\/\/att\/([0-9a-f]+)\)/g)].length
  return {
    preview: previewFromMarkdown(note),
    cover: coverFromMarkdown(note).slice(0, 4).map((hash) => ({ hash, width: previewFiles.get(hash)?.width ?? null, height: previewFiles.get(hash)?.height ?? null })),
    image_count: images,
    file_count: [...note.matchAll(/\[📎[^\]]*\]\(ideario:\/\/att\//g)].length,
    hash_tags: [...new Set(hashTags(note))],
  }
}

/** Quantos tópicos há debaixo de um (os que somem quando ele está recolhido). */
function descendants(id: number): number {
  const n = nodes.get(id)
  return n ? n.children.reduce((sum, c) => sum + 1 + descendants(c), 0) : 0
}


// Backend simulado para o modo preview.
//
// O app real fala com o Rust por comandos Tauri. No navegador não há Rust, e
// `invoke` lança. Este módulo implementa os mesmos comandos sobre um mapa em
// memória, com a mesma forma de snapshot, para que a interface inteira possa
// ser vista e ajustada sem compilar o aplicativo.
//
// Ativado por `?preview` na URL, ou automaticamente quando não há host Tauri e
// o Vite está em modo de desenvolvimento.

import type { NodeSize, Snapshot } from './api'

// ── modelo em memória ────────────────────────────────────────────────────

interface PNode {
  id: number
  text: string
  note: string
  side: 'left' | 'right'
  color: string | null
  colorToken: string | null
  folded: boolean
  parent: number | null
  children: number[]
  x: number
  y: number
  width: number
  height: number
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
  })
}

function restore(state: string) {
  const o = JSON.parse(state) as {
    nodes: [number, PNode][]
    nextId: number
    rootId: number
    dirty: boolean
    path: string | null
  }
  nodes = new Map(o.nodes)
  nextId = o.nextId
  rootId = o.rootId
  dirty = o.dirty
  path = o.path
}

function snapshotOf(): Snapshot {
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
      dirty,
      path,
      title: 'Sem título',
      can_undo: past.length > 0,
      can_redo: future.length > 0,
      stale: false,
      bounds: { min_x: 0, min_y: 0, max_x: 0, max_y: 0 },
    }
  }

  let minX = Infinity
  let minY = Infinity
  let maxX = -Infinity
  let maxY = -Infinity

  const flat = list.map((n) => {
    minX = Math.min(minX, n.x)
    minY = Math.min(minY, n.y)
    maxX = Math.max(maxX, n.x + n.width)
    maxY = Math.max(maxY, n.y + n.height)
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
      child_ids: [...n.children],
      parent: n.parent,
      has_children: n.children.length > 0,
    }
  })

  // Pré-ordem: o inspetor e o desenho esperam a raiz primeiro.
  flat.sort((a, b) => a.depth - b.depth || a.id - b.id)

  return {
    nodes: flat,
    connections: [],
    dirty,
    path,
    title: nodes.get(rootId)?.text ?? 'Sem título',
    can_undo: past.length > 0,
    can_redo: future.length > 0,
    stale: false,
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

// ── layout: espelha mapist-core/src/layout.rs ────────────────────────────

function measure(n: PNode) {
  const lines = n.text.split('\n')
  const longest = Math.max(...lines.map((l) => l.length), 1)
  n.width = Math.max(MIN_W, longest * CHAR_W + 36)
  n.height = Math.max(MIN_H, lines.length * 19 + 22)
}

/** Altura de que uma sub-árvore precisa, para irmãos nunca se sobreporem (layout.rs: `extent`). */
function extent(n: PNode): number {
  if (n.children.length === 0 || n.folded) return n.height
  const band = (ids: number[]) =>
    ids.length === 0 ? 0 : ids.reduce((a, id) => a + extent(nodes.get(id)!), 0) + V_GAP * (ids.length - 1)
  const kids = n.children.filter((id) => nodes.has(id))
  const left = kids.filter((id) => nodes.get(id)!.side === 'left')
  const right = kids.filter((id) => nodes.get(id)!.side !== 'left')
  return Math.max(n.height, band(left), band(right))
}

/** Posiciona `n` numa faixa que começa em `top`, com a borda próxima em `x` (layout.rs: `place`). */
function place(n: PNode, x: number, top: number) {
  const ext = extent(n)
  n.x = x
  n.y = top + (ext - n.height) / 2
  if (n.children.length === 0 || n.folded) return
  for (const left of [true, false]) {
    const group = n.children.map((id) => nodes.get(id)!).filter((c) => c && (c.side === 'left') === left)
    if (group.length === 0) continue
    const total = group.reduce((a, c) => a + extent(c), 0) + V_GAP * (group.length - 1)
    let cursor = top + (ext - total) / 2
    for (const c of group) {
      place(c, left ? n.x - H_GAP - c.width : n.x + n.width + H_GAP, cursor)
      cursor += extent(c) + V_GAP
    }
  }
}

/** Mede tudo pela estimativa e posiciona (layout.rs: `layout`). */
function relayout() {
  for (const n of nodes.values()) measure(n)
  reposition()
}

/** Só posiciona, com os tamanhos que já estão nos nós (layout.rs: `reposition`). */
function reposition() {
  const root = nodes.get(rootId)
  if (root) place(root, 0, -extent(root) / 2)
}

// ── histórico ────────────────────────────────────────────────────────────

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

export function seedDemo() {
  nodes.clear()
  nextId = 1
  past.length = 0
  future.length = 0
  dirty = false
  path = 'exemplo.minder'

  // Um mapa inventado (o repositório é público): o lançamento de um app fictício, com ramos dos dois lados.
  const root = makeNode('Lançamento do app', 'right', 'Plano do trimestre. **Responsável:** time de produto.')
  rootId = root.id
  root.parent = null
  nodes.set(root.id, root)

  const branch = (parent: PNode, text: string, token: string, children: string[] = [], side = parent.side) => {
    const n = makeNode(text, side, '', token)
    n.parent = parent.id
    n.color = token
    parent.children.push(n.id)
    nodes.set(n.id, n)
    for (const c of children) branch(n, c, token)
    return n
  }

  branch(root, 'Descoberta', '#6eb0f7', ['Entrevistas', 'Análise de mercado'])
  const design = branch(root, 'Design', '#b28ee6', ['Wireframes', 'Design system'])
  const ds = nodes.get(design.children[1])!
  branch(ds, 'Cores', '#b28ee6')
  branch(ds, 'Componentes', '#b28ee6')
  const build = branch(root, 'Construção', '#7fbf7f', ['Núcleo', 'Layout', 'Servidor MCP', 'Janela'])
  branch(root, 'Lançamento', '#e8c547', ['Beta', 'Documentação', 'Versão 1.0'], 'left')
  branch(root, 'Divulgação', '#f0a36b', ['Site', 'Vídeo curto', 'Comunidade'], 'left')
  branch(root, 'Pesquisa', '#e5818d', ['Concorrentes', 'Artigos'], 'left')

  // Uma nota e um ramo recolhido, para os marcadores aparecerem.
  nodes.get(build.children[2])!.note = 'Dezessete ferramentas, o mesmo desfazer da janela.'
  nodes.get(root.children[root.children.length - 1])!.folded = true

  relayout()
}

// ── comandos ─────────────────────────────────────────────────────────────

export const previewBackend = {
  get_snapshot: () => snapshotOf(),

  new_map: ({ title }: { title: string }) => {
    nodes.clear()
    nextId = 1
    past.length = 0
    future.length = 0
    const root = makeNode(title || 'Tópico central')
    rootId = root.id
    nodes.set(root.id, root)
    dirty = false
    path = null
    relayout()
    return snapshotOf()
  },

  open_map: ({ path: p }: { path: string }) => {
    seedDemo()
    path = p
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
      n.colorToken = parentNode.colorToken
      n.color = parentNode.colorToken
      nodes.set(n.id, n)
      parentNode.children.push(n.id)
      newId = n.id
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

  set_sizes: ({ sizes }: { sizes: NodeSize[] }) => {
    for (const s of sizes) {
      const n = nodes.get(s.id)
      if (n) {
        n.width = s.width
        n.height = s.height
      }
    }
    relayout()
    return snapshotOf()
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
    command: '/Applications/Mapist.app/Contents/MacOS/mapist',
    args: ['--mcp'],
    claudeCode: 'claude mcp add --scope user mapist -- "/Applications/Mapist.app/Contents/MacOS/mapist" --mcp',
    desktopConfig: '~/Library/Application Support/Claude/claude_desktop_config.json',
    desktopFound: true,
    desktopInstalled: false,
  }),

  mcp_install: () => previewBackend.mcp_info(),
  mcp_uninstall: () => previewBackend.mcp_info(),

  log_measurements: () => undefined,
}

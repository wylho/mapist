// Estado do documento aberto.
//
// Fonte da verdade é o Rust: cada comando devolve um snapshot completo e este
// store só o guarda. Não há estado de mapa duplicado no frontend, o que evita
// divergência entre o que a tela mostra e o que o core acha que tem.

import { api, hasHost, type Snapshot } from './api'
import { measureAll } from './measure'

const EMPTY: Snapshot = {
  nodes: [],
  connections: [],
  dirty: false,
  path: null,
  title: 'Sem título',
  can_undo: false,
  can_redo: false,
  bounds: { min_x: 0, min_y: 0, max_x: 0, max_y: 0 },
}

class Document {
  snapshot = $state<Snapshot>(EMPTY)
  selected = $state<number | null>(null)
  hovered = $state<number | null>(null)
  matches = $state<Set<number>>(new Set())
  error = $state<string | null>(null)
  /** Dimensões medidas no frontend, por id. */
  sizes = $state<Map<number, { width: number; height: number }>>(new Map())
  busy = $state(false)

  get nodes() {
    return this.snapshot.nodes
  }

  get hasDocument() {
    return this.snapshot.nodes.length > 0
  }

  node(id: number | null) {
    if (id === null) return null
    return this.snapshot.nodes.find((n) => n.id === id) ?? null
  }

  get selectedNode() {
    return this.node(this.selected)
  }

  /** Aplica um snapshot novo e reenvia as medições. */
  private async adopt(next: Snapshot) {
    this.snapshot = next
    if (this.selected !== null && !next.nodes.some((n) => n.id === this.selected)) {
      this.selected = next.nodes[0]?.id ?? null
    }
    await this.syncSizes()
  }

  /**
   * Mede os nós aqui e envia ao host quando alguma dimensão mudou. O Rust
   * reposiciona e devolve um snapshot novo; nesse caso não reenviamos, senão
   * entraríamos em laço.
   */
  async syncSizes() {
    if (!hasHost || this.snapshot.nodes.length === 0) return
    const measured = measureAll(this.snapshot.nodes)
    let changed = false
    const next = new Map<number, { width: number; height: number }>()
    for (const m of measured) {
      next.set(m.id, { width: m.width, height: m.height })
      const old = this.snapshot.nodes.find((n) => n.id === m.id)
      if (!old || Math.abs(old.width - m.width) > 0.5 || Math.abs(old.height - m.height) > 0.5) {
        changed = true
      }
    }
    this.sizes = next
    if (!changed) return
    try {
      const updated = await api.setSizes(measured)
      this.snapshot = updated
    } catch (e) {
      // O host pode não expor set_sizes ainda; seguir com a estimativa do core
      // é melhor do que travar a interface.
      console.warn('set_sizes indisponível:', e)
    }
  }

  /** Executa um comando do host tratando erro de forma uniforme. */
  private async run(fn: () => Promise<Snapshot>) {
    if (!hasHost) return
    this.busy = true
    try {
      await this.adopt(await fn())
      this.error = null
    } catch (e) {
      this.error = String(e)
      console.error(e)
    } finally {
      this.busy = false
    }
  }

  async load() {
    if (!hasHost) return
    try {
      const snap = await api.snapshot()
      if (snap.nodes.length > 0) await this.adopt(snap)
      else await this.create('Tópico central')
    } catch (e) {
      this.error = String(e)
    }
  }

  create(title: string) {
    return this.run(() => api.newMap(title))
  }

  open(path: string) {
    return this.run(() => api.open(path))
  }

  save(path?: string) {
    return this.run(() => api.save(path))
  }

  addChild(text = 'Nova ideia') {
    const id = this.selected
    if (id === null) return Promise.resolve()
    return this.run(() => api.addNode(id, text))
  }

  addSibling(text = 'Nova ideia') {
    const n = this.selectedNode
    if (!n || n.parent === null) return Promise.resolve()
    const parent = n.parent
    return this.run(() => api.addNode(parent, text))
  }

  remove() {
    const n = this.selectedNode
    if (!n || n.parent === null) return Promise.resolve()
    const id = n.id
    return this.run(() => api.removeNode(id))
  }

  setText(id: number, text: string) {
    return this.run(() => api.setText(id, text))
  }

  setNote(id: number, note: string) {
    return this.run(() => api.setNote(id, note))
  }

  setColor(id: number, color: string | null) {
    return this.run(() => api.setColor(id, color))
  }

  /** Persiste a posição final de um nó arrastado. */
  move(id: number, x: number, y: number) {
    return this.run(() => api.moveNode(id, x, y))
  }

  toggleFold() {
    const id = this.selected
    if (id === null) return Promise.resolve()
    return this.run(() => api.toggleFold(id))
  }

  undo() {
    return this.run(() => api.undo())
  }

  redo() {
    return this.run(() => api.redo())
  }

  async search(query: string) {
    if (!hasHost) return
    const q = query.trim()
    if (!q) {
      this.matches = new Set()
      return
    }
    try {
      this.matches = new Set(await api.search(q))
    } catch (e) {
      console.error(e)
    }
  }

  /** Move a seleção para o vizinho mais próximo na direção dada. */
  moveSelection(dx: number, dy: number) {
    const cur = this.selectedNode
    if (!cur) return
    const cx = cur.x + cur.width / 2
    const cy = cur.y + cur.height / 2

    let best: number | null = null
    let bestScore = Infinity
    for (const n of this.nodes) {
      if (n.id === cur.id) continue
      const nx = n.x + n.width / 2
      const ny = n.y + n.height / 2
      const ddx = nx - cx
      const ddy = ny - cy
      if (dx !== 0 && Math.sign(ddx) !== Math.sign(dx)) continue
      if (dy !== 0 && Math.sign(ddy) !== Math.sign(dy)) continue
      const along = dx !== 0 ? Math.abs(ddx) : Math.abs(ddy)
      const across = dx !== 0 ? Math.abs(ddy) : Math.abs(ddx)
      const score = along + across * 3
      if (score < bestScore) {
        bestScore = score
        best = n.id
      }
    }
    if (best !== null) this.selected = best
  }
}

export const doc = new Document()

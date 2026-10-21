// Estado do documento aberto.
//
// Fonte da verdade é o Rust: cada comando devolve um snapshot completo e este
// store só o guarda. Não há estado de mapa duplicado no frontend, o que evita
// divergência entre o que a tela mostra e o que o core acha que tem.

import { api, hasHost, isPreview, type Snapshot } from './api'
import { measureAll } from './measure'
import { mapTheme } from './maptheme'
import { ui } from './ui.svelte'

const EMPTY: Snapshot = {
  nodes: [],
  connections: [],
  dirty: false,
  path: null,
  title: 'Sem título',
  can_undo: false,
  can_redo: false,
  stale: false,
  style: null,
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
  /**
   * Nó sendo editado no lugar, como no XMind. `draft` é o texto no campo; `selectAll` diz se ele abre com tudo
   * selecionado (Espaço, F2, nó novo) ou com o cursor no fim (começou digitando uma letra).
   */
  editing = $state<{ id: number; draft: string; selectAll: boolean } | null>(null)
  /** Um nó novo está nascendo (Tab/Enter): o que se digita até ele existir fica em `typed` e vai para ele. */
  creating = false
  private typed = ''

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
    this.syncTitle()
    // Sempre há um nó selecionado quando há mapa: dá para começar a digitar sem clicar antes.
    if (this.selected === null || !next.nodes.some((n) => n.id === this.selected)) this.selected = next.nodes[0]?.id ?? null
    await this.syncSizes()
  }

  /** Título da janela: o nome do arquivo (ou do mapa), com • quando há algo por salvar. */
  private syncTitle() {
    if (!hasHost) return
    const s = this.snapshot
    const name = s.path ? s.path.split(/[\\/]/).pop() : s.title || 'Sem título'
    const title = `${s.dirty ? '• ' : ''}${name} — Nodo`
    import('@tauri-apps/api/window')
      .then(({ getCurrentWindow }) => getCurrentWindow().setTitle(title))
      .catch(() => {})
  }

  /**
   * Mede os nós aqui e envia ao host quando alguma dimensão mudou. O Rust
   * reposiciona e devolve um snapshot novo; nesse caso não reenviamos, senão
   * entraríamos em laço.
   */
  async syncSizes() {
    if ((!hasHost && !isPreview) || this.snapshot.nodes.length === 0) return
    const measured = measureAll(this.snapshot.nodes, mapTheme(this.snapshot.style))
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
    if (!hasHost && !isPreview) return
    this.busy = true
    try {
      await this.adopt(await fn())
      this.error = null
    } catch (e) {
      this.error = String(e)
      ui.say(String(e instanceof Error ? e.message : e))
      console.error(e)
    } finally {
      this.busy = false
    }
  }

  async load() {
    if (!hasHost && !isPreview) return
    try {
      const snap = await api.snapshot()
      if (snap.nodes.length > 0) await this.adopt(snap)
      else await this.create('Tópico central')
      // Prévia: `?select=Texto do nó` já abre com ele selecionado (prints do site, scripts/site-shots.mjs).
      const pick = isPreview ? new URLSearchParams(location.search).get('select') : null
      if (pick) this.selected = this.snapshot.nodes.find((n) => n.text === pick)?.id ?? null
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

  async save(path?: string) {
    await this.run(() => api.save(path))
    if (!this.error) ui.say('Mapa salvo')
  }

  /**
   * Pergunta antes de descartar alterações não salvas, no diálogo de
   * confirmação do Ideario.
   */
  async confirmDiscard(): Promise<boolean> {
    if (!this.snapshot.dirty) return true
    return ui.ask({
      title: 'Descartar alterações?',
      text: 'Este mapa tem alterações que não foram salvas.',
      confirm: 'Descartar',
    })
  }

  /** Novo mapa (⌘N), depois de confirmar o descarte. */
  async newMap() {
    if (await this.confirmDiscard()) await this.create('Tópico central')
  }

  /** Abre um arquivo vindo de fora (Finder), depois de confirmar o descarte. */
  async openPath(path: string) {
    if (await this.confirmDiscard()) await this.open(path)
  }

  /** Abrir (⌘O): diálogo nativo do sistema; no preview, um prompt. */
  async openDialog() {
    if (!(await this.confirmDiscard())) return
    let path: string | null = null
    if (hasHost) {
      const { open } = await import('@tauri-apps/plugin-dialog')
      const picked = await open({ multiple: false, filters: [{ name: 'Mapa mental', extensions: ['minder'] }] })
      path = typeof picked === 'string' ? picked : null
    } else {
      path = window.prompt('Caminho do arquivo .minder')
    }
    if (path) await this.open(path)
  }

  /** Salvar (⌘S): grava no arquivo atual; mapa novo ou `as` pede o caminho. */
  async saveDialog(as = false) {
    if (this.snapshot.path && !as) return this.save()
    let path: string | null = null
    if (hasHost) {
      const { save } = await import('@tauri-apps/plugin-dialog')
      path = await save({
        defaultPath: `${(this.snapshot.title || 'mapa').replace(/[\\/:*?"<>|]+/g, ' ').trim() || 'mapa'}.minder`,
        filters: [{ name: 'Mapa mental', extensions: ['minder'] }],
      })
    } else {
      path = window.prompt('Salvar como (caminho)')
    }
    if (path) await this.save(path.endsWith('.minder') ? path : `${path}.minder`)
  }

  /** Abre a edição no nó selecionado. Com `initial`, começa com esse texto (a letra que a pessoa digitou). */
  startEdit(initial?: string) {
    const n = this.selectedNode
    if (!n) return
    this.editing = initial === undefined
      ? { id: n.id, draft: n.text, selectAll: true }
      : { id: n.id, draft: initial, selectAll: false }
  }

  /**
   * Uma letra digitada fora do campo de texto. Enquanto um nó novo nasce, fica guardada para ele; com o campo já
   * aberto (o foco chega no quadro seguinte), entra no rascunho; senão começa a editar o nó selecionado. Assim quem
   * digita rápido depois de Tab ou Enter não perde letras nem escreve no nó anterior.
   */
  type(ch: string) {
    if (this.creating) {
      this.typed += ch
      return
    }
    const ed = this.editing
    if (ed) {
      this.editing = { ...ed, draft: ed.selectAll ? ch : ed.draft + ch, selectAll: false }
      return
    }
    this.startEdit(ch)
  }

  /** Grava o que está no campo. Texto vazio não apaga o nó: mantém o que havia. */
  async commitEdit() {
    const ed = this.editing
    this.editing = null
    if (!ed) return
    const n = this.node(ed.id)
    const next = ed.draft.trim()
    if (!n || next === '' || next === n.text) return
    await this.setText(ed.id, next)
  }

  cancelEdit() {
    this.editing = null
  }

  /** Cria um nó, seleciona e já abre a edição com o texto provisório selecionado (como no XMind). */
  private async addAndEdit(parent: number, text: string, unfoldFirst = false) {
    this.creating = true
    this.typed = ''
    let created
    try {
      // Filho de um nó recolhido ficaria escondido: abre o ramo antes.
      if (unfoldFirst) await this.run(() => api.toggleFold(parent))
      const before = new Set(this.snapshot.nodes.map((n) => n.id))
      await this.run(() => api.addNode(parent, text))
      created = this.snapshot.nodes.find((n) => !before.has(n.id))
    } finally {
      this.creating = false
    }
    const typed = this.typed
    this.typed = ''
    if (!created) return
    this.selected = created.id
    this.editing = typed
      ? { id: created.id, draft: typed, selectAll: false }
      : { id: created.id, draft: created.text, selectAll: true }
  }

  addChild(text = 'Nova ideia') {
    const id = this.selected
    if (id === null) return Promise.resolve()
    return this.addAndEdit(id, text, !!this.selectedNode?.folded)
  }

  addSibling(text = 'Nova ideia') {
    const n = this.selectedNode
    // Na raiz não há irmão: Enter cria um filho, como no XMind.
    if (!n) return Promise.resolve()
    if (n.parent === null) return this.addChild(text)
    return this.addAndEdit(n.parent, text)
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

  /** Tema do mapa (fica no arquivo e desfaz como qualquer edição). */
  setMapStyle(style: string | null) {
    return this.run(() => api.setMapStyle(style))
  }

  setSide(id: number, side: 'left' | 'right') {
    return this.run(() => api.setSide(id, side))
  }

  /**
   * Um agente pode salvar o mesmo arquivo pelo MCP. Ao voltar para a janela, o mapa é relido: direto se não há
   * nada por salvar aqui, ou depois de perguntar, porque reler descartaria o que foi feito na janela.
   */
  async checkDisk() {
    if (!hasHost || this.busy || !this.snapshot.path) return
    let snap: Snapshot
    try {
      snap = await api.snapshot()
    } catch {
      return
    }
    if (!snap.stale) return
    if (snap.dirty) {
      const ok = await ui.ask({
        title: 'O arquivo mudou fora do Nodo',
        text: 'Outro programa (talvez o Claude) salvou este mapa. Recarregar descarta as alterações feitas aqui.',
        confirm: 'Recarregar',
      })
      if (!ok) return
    }
    await this.run(() => api.reload())
    if (!this.error) ui.say('Mapa atualizado a partir do arquivo')
  }

  /** A janela pediu para fechar com alterações não salvas. */
  async confirmQuit() {
    const ok = await ui.ask({
      title: 'Sair sem salvar?',
      text: 'Este mapa tem alterações que não foram salvas. Para guardá-las, cancele e salve antes.',
      confirm: 'Sair sem salvar',
    })
    if (ok) await api.quit()
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
    if (!hasHost && !isPreview) return
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

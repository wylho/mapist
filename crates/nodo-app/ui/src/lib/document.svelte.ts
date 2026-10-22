// Estado do documento aberto.
//
// Fonte da verdade é o Rust: cada comando devolve um snapshot completo e este
// store só o guarda. Não há estado de mapa duplicado no frontend, o que evita
// divergência entre o que a tela mostra e o que o core acha que tem.

import { api, hasHost, isPreview, type CardFields, type Edit, type Snapshot } from './api'
import { t, tn } from '@wylho/atrio'
import { measureAll } from './measure'
import { mapTheme } from './maptheme'
import { ui } from './ui.svelte'
import { recents } from './recents.svelte'
import { notes } from './notes.svelte'
import { emptyMeta, setStatus, type ClipCard, type NoteColor, type ReminderStatus, type RepeatKind } from '@wylho/atrio'
import { NODE_COLOR_TOKENS, resolveNodeColor } from './palettes'
import { theme } from './theme.svelte'

/** A cor do ramo, guardada já resolvida (hex do tema), como a cor de nota do mesmo lugar na paleta; outra é `none`. */
export function noteColorOf(color: string | null): NoteColor {
  const i = color ? theme.colors.notes.findIndex((c) => c.toLowerCase() === color.toLowerCase()) : -1
  return i >= 0 ? (NODE_COLOR_TOKENS[i].slice(4) as NoteColor) : 'none'
}

/** A cor de nota como cor do ramo (hex do tema), ou null para `none`. */
export function nodeColorOf(color: NoteColor | null | undefined): string | null {
  return color && color !== 'none' ? resolveNodeColor(`--n-${color}`, theme.colors) : null
}

const EMPTY: Snapshot = {
  nodes: [],
  notes: [],
  connections: [],
  dirty: false,
  path: null,
  title: 'Sem título',
  can_undo: false,
  can_redo: false,
  stale: false,
  style: null,
  structure: null,
  spacing: null,
  bounds: { min_x: 0, min_y: 0, max_x: 0, max_y: 0 },
  tabs: [],
  active: null,
}

/** Os campos do card com o status trocado, pela regra do Atrio (`setStatus`). */
function withStatus(c: CardFields, to: ReminderStatus | 'none'): CardFields {
  const m = emptyMeta({
    reminderAt: c.reminder_at ?? null,
    reminderRepeat: (c.reminder_repeat ?? null) as RepeatKind | null,
    reminderDone: !!c.reminder_done,
    reminderDoing: !!c.reminder_doing,
    statusOn: !!c.status_on,
  })
  setStatus(m, to)
  return {
    ...c,
    reminder_at: m.reminderAt,
    reminder_repeat: m.reminderRepeat,
    reminder_done: m.reminderDone,
    reminder_doing: m.reminderDoing,
    status_on: m.statusOn,
  }
}

class Document {
  snapshot = $state<Snapshot>(EMPTY)
  private _selected = $state<number | null>(null)
  /**
   * Seleção de vários (arrastar um retângulo, ⇧/⌘ + clique): os tópicos e as notas soltas do grupo. Vazia quando só
   * há o tópico principal. O principal (`selected`) é sempre um deles: digitar, Tab e o card aberto valem para ele.
   */
  picked = $state<Set<number>>(new Set())
  pickedNotes = $state<Set<number>>(new Set())

  /** O tópico principal. Escolher um tópico assim desfaz o grupo. */
  get selected() {
    return this._selected
  }
  set selected(id: number | null) {
    this._selected = id
    if (this.picked.size || this.pickedNotes.size) this.clearGroup()
  }
  hovered = $state<number | null>(null)
  matches = $state<Set<number>>(new Set())
  error = $state<string | null>(null)
  /** Dimensões medidas no frontend, por id. */
  sizes = $state<Map<number, { width: number; height: number }>>(new Map())
  busy = $state(false)
  /** O primeiro snapshot já chegou (antes disso, nem mapa nem tela inicial). */
  loaded = $state(false)
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
    if (this.picked.size || this.pickedNotes.size) {
      const ids = new Set(next.nodes.map((n) => n.id))
      const notes = new Set(next.notes.map((n) => n.id))
      this.picked = new Set([...this.picked].filter((id) => ids.has(id)))
      this.pickedNotes = new Set([...this.pickedNotes].filter((id) => notes.has(id)))
    }
    if (this._selected === null || !next.nodes.some((n) => n.id === this._selected)) this._selected = next.nodes[0]?.id ?? null
    await this.syncSizes()
  }

  /** Título da janela: o nome do arquivo (ou do mapa), com • quando há algo por salvar. */
  private syncTitle() {
    if (!hasHost) return
    const s = this.snapshot
    const name = s.path ? s.path.split(/[\\/]/).pop() : s.title || t('Sem título')
    const title = s.active === null ? 'Nodo' : `${s.dirty ? '• ' : ''}${name} — Nodo`
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
      // Sem arquivo para abrir, o app começa na tela inicial (Novo, Abrir e os recentes), não num mapa vazio.
      await this.adopt(snap)
      // Prévia: `?select=Texto do nó` já abre com ele selecionado (prints do site, scripts/site-shots.mjs).
      const pick = isPreview ? new URLSearchParams(location.search).get('select') : null
      if (pick) this.selected = this.snapshot.nodes.find((n) => n.text === pick)?.id ?? null
    } catch (e) {
      this.error = String(e)
    } finally {
      this.loaded = true
    }
  }

  create(title: string) {
    return this.run(() => api.newMap(title))
  }

  /** Abre um .nodo, ou importa um .minder antigo, Markdown ou OPML como mapa novo (que pergunta onde salvar). */
  async open(path: string) {
    await this.run(() => api.open(path))
    if (!this.error) recents.add(path)
    const name = path.split(/[\\/]/).pop() ?? path
    if (!this.error && /\.(md|markdown|txt|opml|minder)$/i.test(path)) {
      ui.say(t('{nome} virou um mapa. Salve para guardá-lo como .nodo; o original fica como está.', { nome: name }), 5200)
    }
  }

  async save(path?: string) {
    await this.run(() => api.save(path))
    if (this.error) return
    if (this.snapshot.path) recents.add(this.snapshot.path)
    ui.say(t('Mapa salvo'))
  }

  /** Zoom e posição de cada aba, para voltar a ela como estava (o Canvas guarda e lê). */
  views = new Map<number, { x: number; y: number; scale: number }>()
  /** O nó selecionado em cada aba. */
  private selections = new Map<number, number | null>()

  /** Os mapas abertos, na ordem das abas. */
  get tabs() {
    return this.snapshot.tabs
  }

  /** Nome que a aba mostra: o do arquivo, ou o do mapa enquanto não foi salvo. */
  tabName(tab: { title: string; path: string | null }) {
    return tab.path ? (tab.path.split(/[\\/]/).pop() ?? tab.path).replace(/\.nodo$/i, '') : tab.title || t('Sem título')
  }

  /** Antes de outra aba vir para a frente: guarda a seleção desta e fecha o que é só dela. */
  private async leaveTab() {
    if (this.editing) await this.commitEdit()
    const from = this.snapshot.active
    if (from !== null) this.selections.set(from, this.selected)
    notes.close()
  }

  /** Traz uma aba para a frente, com a seleção que ela tinha. */
  async switchTab(id: number) {
    if (id === this.snapshot.active) return
    await this.leaveTab()
    this.selected = this.selections.get(id) ?? null
    await this.run(() => api.switchTab(id))
  }

  /** A aba seguinte (ou a anterior, com `step` -1), dando a volta. */
  cycleTab(step: 1 | -1) {
    const list = this.snapshot.tabs
    const i = list.findIndex((t) => t.id === this.snapshot.active)
    if (list.length < 2 || i < 0) return Promise.resolve()
    return this.switchTab(list[(i + step + list.length) % list.length].id)
  }

  /** Salvar, fechar sem salvar ou cancelar, para o mapa da frente. `true` se pode seguir. */
  private async settleUnsaved(discard: string): Promise<boolean> {
    if (!this.snapshot.dirty) return true
    const name = this.tabName(this.snapshot)
    const choice = await ui.choose({
      title: t('Salvar as alterações?'),
      text: t('“{nome}” tem alterações que não foram salvas.', { nome: name }),
      save: t('Salvar'),
      discard,
    })
    if (choice === 'cancel') return false
    if (choice === 'discard') return true
    await this.saveDialog()
    return !this.snapshot.dirty
  }

  /** Fecha uma aba (⌘W ou o × dela). Com algo por salvar, pergunta antes; sem abas, volta a tela inicial. */
  async closeTab(id = this.snapshot.active) {
    const tab = this.snapshot.tabs.find((t) => t.id === id)
    if (!tab || id === null) return
    if (tab.dirty) {
      await this.switchTab(id)
      if (!(await this.settleUnsaved(t('Fechar sem salvar')))) return
    }
    if (id === this.snapshot.active) await this.leaveTab()
    this.selections.delete(id)
    this.views.delete(id)
    await this.run(() => api.closeTab(id))
    // A aba que veio para a frente volta com a seleção que tinha.
    const back = this.snapshot.active !== null ? this.selections.get(this.snapshot.active) : null
    if (back != null && this.snapshot.nodes.some((n) => n.id === back)) this.selected = back
  }

  /** Novo mapa (⌘N), numa aba nova. */
  async newMap() {
    await this.leaveTab()
    await this.create(t('Tópico central'))
  }

  /** Abre um arquivo vindo de fora (Finder), numa aba nova (ou na dele, se já estiver aberto). */
  async openPath(path: string) {
    await this.leaveTab()
    await this.open(path)
  }

  /** Abrir (⌘O): diálogo nativo do sistema; no preview, um prompt. O arquivo vem numa aba nova. */
  async openDialog() {
    let path: string | null = null
    if (hasHost) {
      const { open } = await import('@tauri-apps/plugin-dialog')
      const picked = await open({
        multiple: false,
        filters: [
          { name: t('Mapas e listas'), extensions: ['nodo', 'minder', 'md', 'markdown', 'opml', 'txt'] },
          { name: t('Mapa do Nodo'), extensions: ['nodo'] },
          { name: 'Markdown', extensions: ['md', 'markdown', 'txt'] },
          { name: 'OPML', extensions: ['opml'] },
        ],
      })
      path = typeof picked === 'string' ? picked : null
    } else {
      path = window.prompt(t('Caminho do arquivo (.nodo, .minder, .md ou .opml)'))
    }
    if (path) await this.openPath(path)
  }

  /** Salvar (⌘S): grava no arquivo atual; mapa novo ou `as` pede o caminho. */
  async saveDialog(as = false) {
    if (this.snapshot.path && !as) return this.save()
    let path: string | null = null
    if (hasHost) {
      const { save } = await import('@tauri-apps/plugin-dialog')
      path = await save({
        defaultPath: `${(this.snapshot.title || 'mapa').replace(/[\\/:*?"<>|]+/g, ' ').trim() || 'mapa'}.nodo`,
        filters: [{ name: t('Mapa do Nodo'), extensions: ['nodo'] }],
      })
    } else {
      path = window.prompt(t('Salvar como (caminho)'))
    }
    if (path) await this.save(path.endsWith('.nodo') ? path : `${path}.nodo`)
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

  addChild(text = t('Nova ideia')) {
    const id = this.selected
    if (id === null) return Promise.resolve()
    return this.addAndEdit(id, text, !!this.selectedNode?.folded)
  }

  addSibling(text = t('Nova ideia')) {
    const n = this.selectedNode
    // Na raiz não há irmão: Enter cria um filho, como no XMind.
    if (!n) return Promise.resolve()
    if (n.parent === null) return this.addChild(text)
    return this.addAndEdit(n.parent, text)
  }

  /** Sobe ou desce o nó selecionado entre os irmãos (⌥↑ / ⌥↓, como no XMind). Na ponta, não faz nada. */
  async reorder(delta: number) {
    const n = this.selectedNode
    if (!n || n.parent === null) return
    const id = n.id
    try {
      await this.adopt(await api.reorder(id, delta))
    } catch {
      // Já é o primeiro ou o último do seu lado: fica onde está.
    }
  }

  /** Seleciona a raiz (Home). */
  selectRoot() {
    const root = this.snapshot.nodes.find((n) => n.parent === null)
    if (root) this.selected = root.id
  }

  // ── seleção de vários ────────────────────────────────────────────────

  /** Há um grupo (mais de um item escolhido). */
  get grouped() {
    return this.picked.size + this.pickedNotes.size > 1
  }

  /** Os tópicos escolhidos: o grupo, ou só o principal. */
  get selection(): number[] {
    if (this.picked.size || this.pickedNotes.size) return [...this.picked]
    return this._selected !== null ? [this._selected] : []
  }

  /** O tópico está escolhido (principal ou no grupo). */
  isPicked(id: number) {
    return this.picked.size || this.pickedNotes.size ? this.picked.has(id) : this._selected === id
  }

  /** Muda o tópico principal sem desfazer o grupo (clicar num tópico do grupo para arrastar). */
  focus(id: number) {
    this._selected = id
  }

  clearGroup() {
    this.picked = new Set()
    this.pickedNotes = new Set()
  }

  /** ⇧/⌘ + clique: soma ou tira um tópico (ou nota solta) do grupo. */
  toggle(id: number, note = false) {
    const picked = new Set(this.picked.size || this.pickedNotes.size || this._selected === null ? this.picked : [this._selected])
    const notes = new Set(this.pickedNotes)
    const set = note ? notes : picked
    if (set.has(id)) set.delete(id)
    else set.add(id)
    this.setGroup(picked, notes, note ? undefined : id)
  }

  /** O retângulo: estes tópicos e notas (somados ao grupo atual com `add`). */
  pickArea(ids: number[], noteIds: number[], add: boolean) {
    const base = add ? new Set(this.selection) : new Set<number>()
    const picked = new Set([...base, ...ids])
    const notes = new Set([...(add ? this.pickedNotes : []), ...noteIds])
    this.setGroup(picked, notes, ids[0])
  }

  /** ⌘A: todos os tópicos e notas soltas. */
  pickAll() {
    this.setGroup(new Set(this.nodes.map((n) => n.id)), new Set(this.snapshot.notes.map((n) => n.id)), this._selected ?? undefined)
  }

  private setGroup(picked: Set<number>, notes: Set<number>, primary?: number) {
    if (picked.size + notes.size <= 1 && notes.size === 0) {
      // Um tópico só (ou nenhum): volta a ser a seleção simples.
      this.clearGroup()
      const only = [...picked][0]
      if (only !== undefined) this._selected = only
      return
    }
    this.picked = picked
    this.pickedNotes = notes
    if (primary !== undefined && picked.has(primary)) this._selected = primary
    else if (this._selected === null || !picked.has(this._selected)) this._selected = [...picked][0] ?? this._selected
  }

  /** Apaga o grupo (a raiz fica), num passo só. */
  removeGroup() {
    const root = this.nodes.find((n) => n.parent === null)?.id
    const edits: Edit[] = [
      ...this.selection.filter((id) => id !== root).map((id) => ({ kind: 'remove' as const, id })),
      ...[...this.pickedNotes].map((id) => ({ kind: 'removeFree' as const, id })),
    ]
    if (!edits.length) return Promise.resolve()
    const parent = this.node(this._selected)?.parent ?? root ?? null
    return this.run(() => api.applyEdits(edits)).then(() => {
      this.clearGroup()
      if (parent !== null && this.node(parent)) this._selected = parent
    })
  }

  /** Cor do ramo de todos os tópicos do grupo. */
  colorGroup(color: string | null) {
    return this.run(() => api.applyEdits(this.selection.map((id) => ({ kind: 'color', id, color }))))
  }

  /** Status de todos os tópicos do grupo, cada um pela regra do Atrio. */
  statusGroup(to: ReminderStatus | 'none') {
    return this.run(() =>
      api.applyEdits(this.selection.map((id) => ({ kind: 'card', id, card: withStatus(this.node(id)?.card ?? {}, to) }))),
    )
  }

  /** Arrastar o grupo: cada tópico (com seus ramos) e nota solta anda o mesmo tanto. */
  moveGroup(dx: number, dy: number) {
    const edits: Edit[] = []
    for (const id of this.selection) {
      const n = this.node(id)
      if (n) edits.push({ kind: 'move', id, x: n.x + dx, y: n.y + dy })
    }
    for (const id of this.pickedNotes) {
      const f = this.freeNote(id)
      if (f) edits.push({ kind: 'placeFree', id, x: Math.round(f.x + dx), y: Math.round(f.y + dy), width: f.width, height: f.height })
    }
    return this.run(() => api.applyEdits(edits))
  }

  /** Os tópicos do grupo como cards do Atrio, na ordem do mapa. */
  groupCards(): ClipCard[] {
    const want = new Set(this.selection)
    return this.nodes.filter((n) => want.has(n.id)).map((n) => this.cardOf(n.id)).filter((c): c is ClipCard => c !== null)
  }

  /** Cola vários cards como filhos do tópico principal (ou da raiz), num passo só, e escolhe os novos. */
  async pasteCards(cards: ClipCard[]) {
    if (cards.length === 1) return this.pasteCard(cards[0])
    const parent = this._selected ?? this.nodes.find((n) => n.parent === null)?.id
    if (parent === undefined || !cards.length) return
    const before = new Set(this.nodes.map((n) => n.id))
    await this.run(() =>
      api.applyEdits(
        cards.map((card) => ({
          kind: 'topic',
          parent,
          text: card.title.trim() || t('Nota'),
          note: card.markdown.trim(),
          color: nodeColorOf(card.color),
          card: { emoji: card.emoji ?? null, tags: card.tags ?? [] },
        })),
      ),
    )
    const fresh = this.nodes.filter((n) => !before.has(n.id)).map((n) => n.id)
    this.setGroup(new Set(fresh), new Set(), fresh[0])
  }

  remove() {
    const n = this.selectedNode
    if (!n || n.parent === null) return Promise.resolve()
    const id = n.id
    return this.run(() => api.removeNode(id))
  }

  /** O nó selecionado como card do Atrio, para copiar (o Ideario cola como nota). */
  selectedCard(): ClipCard | null {
    return this.cardOf(this._selected)
  }

  /** Um tópico como card do Atrio. */
  cardOf(id: number | null): ClipCard | null {
    const n = this.node(id)
    if (!n) return null
    const color = noteColorOf(n.color)
    const { emoji, tags } = n.card ?? {}
    return {
      title: n.text,
      markdown: n.note,
      ...(color !== 'none' ? { color } : {}),
      ...(emoji ? { emoji } : {}),
      ...(tags?.length ? { tags } : {}),
    }
  }

  /** Cola um card (do Ideario, ou de outro nó) como filho do nó selecionado, ou da raiz. Seleciona o novo nó. */
  async pasteCard(card: ClipCard) {
    const parent = this.selected ?? this.snapshot.nodes.find((n) => n.parent === null)?.id
    if (parent === undefined) return
    const before = new Set(this.snapshot.nodes.map((n) => n.id))
    const fields = { emoji: card.emoji ?? null, tags: card.tags ?? [] }
    await this.run(() => api.addTopic(parent, card.title.trim() || t('Nota'), card.markdown.trim(), nodeColorOf(card.color), fields))
    const id = this.snapshot.nodes.find((n) => !before.has(n.id))?.id
    if (id !== undefined) this.selected = id
  }

  setText(id: number, text: string) {
    return this.run(() => api.setText(id, text))
  }

  setNote(id: number, note: string) {
    return this.run(() => api.setNote(id, note))
  }

  /** Etiquetas, emoji, lembrete e status do card do nó. */
  setCard(id: number, card: CardFields) {
    return this.run(() => api.setCard(id, card))
  }

  /** Muda o status pelo seletor do card no mapa, com as regras do card (`setStatus` do Atrio). */
  setStatus(id: number, to: ReminderStatus | 'none') {
    if (this.grouped && this.isPicked(id)) return this.statusGroup(to)
    return this.setCard(id, withStatus(this.node(id)?.card ?? {}, to))
  }

  /** Status de uma nota solta, pela mesma regra do Atrio. */
  setFreeStatus(id: number, to: ReminderStatus | 'none') {
    const f = this.freeNote(id)
    if (!f) return
    return this.setFreeCard(id, f.title, f.color, withStatus(f.card ?? {}, to))
  }

  /** Grava o documento do editor do card como a nota do nó (o núcleo converte para Markdown). */
  setNoteDoc(id: number, body: unknown) {
    return this.run(() => api.setNoteDoc(id, body))
  }

  /** Põe uma nota solta no mapa com o canto em (x, y), em unidades de mundo. Devolve o id dela. */
  async addFreeNote(x: number, y: number, text = '') {
    const ids = new Set(this.snapshot.notes.map((n) => n.id))
    await this.run(() => api.addFreeNote(x, y, text))
    return this.snapshot.notes.find((n) => !ids.has(n.id))?.id ?? null
  }

  setFreeNote(id: number, text: string) {
    return this.run(() => api.setFreeNote(id, text))
  }

  placeFreeNote(id: number, x: number, y: number, width: number, height: number) {
    return this.run(() => api.placeFreeNote(id, x, y, width, height))
  }

  /** A nota solta `id`, como o núcleo mandou. */
  freeNote(id: number) {
    return this.snapshot.notes.find((n) => n.id === id)
  }

  setFreeCard(id: number, title: string, color: string | null, card: CardFields) {
    return this.run(() => api.setFreeCard(id, title, color, card))
  }

  setFreeNoteDoc(id: number, body: unknown) {
    return this.run(() => api.setFreeNoteDoc(id, body))
  }

  removeFreeNote(id: number) {
    return this.run(() => api.removeFreeNote(id))
  }

  setColor(id: number, color: string | null) {
    return this.run(() => api.setColor(id, color))
  }

  /** Persiste a posição final de um nó arrastado. */
  move(id: number, x: number, y: number) {
    return this.run(() => api.moveNode(id, x, y))
  }

  /** Tema do mapa (fica no arquivo e desfaz como qualquer edição). */
  /** Estrutura do mapa ("mapa" grava como o padrão). */
  setStructure(structure: string) {
    return this.run(() => api.setStructure(structure === 'mapa' ? null : structure))
  }

  /** Espaçamento ("normal" grava como o padrão). */
  setSpacing(spacing: string) {
    return this.run(() => api.setSpacing(spacing === 'normal' ? null : spacing))
  }

  /** Arruma tudo de novo pela estrutura, desfazendo os arrastos à mão. */
  relayout() {
    return this.run(() => api.relayout())
  }

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
        title: t('O arquivo mudou fora do Nodo'),
        text: t('Outro programa (talvez o Claude) salvou este mapa. Recarregar descarta as alterações feitas aqui.'),
        confirm: t('Recarregar'),
      })
      if (!ok) return
    }
    await this.run(() => api.reload())
    if (!this.error) ui.say(t('Mapa atualizado a partir do arquivo'))
  }

  /** A janela pediu para fechar com alterações não salvas: salvar (todas as abas que têm), sair ou cancelar. */
  async confirmQuit() {
    const dirty = this.snapshot.tabs.filter((t) => t.dirty)
    const many = dirty.length > 1
    const choice = await ui.choose({
      title: t('Salvar as alterações?'),
      text: many
        ? tn(dirty.length, '{n} mapa tem alterações que não foram salvas.', '{n} mapas têm alterações que não foram salvas.')
        : t('“{nome}” tem alterações que não foram salvas.', { nome: this.tabName(dirty[0] ?? this.snapshot) }),
      save: many ? t('Salvar todos') : t('Salvar'),
      discard: t('Sair sem salvar'),
    })
    if (choice === 'cancel') return
    if (choice === 'save') {
      for (const t of dirty) {
        await this.switchTab(t.id)
        await this.saveDialog()
        // Desistiu de escolher onde salvar: fica no app, com a aba à vista.
        if (this.snapshot.dirty) return
      }
    }
    await api.quit()
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

// Só na prévia do navegador: o documento à mão para os testes de ponta a ponta (scripts/, Playwright).
if (isPreview && typeof window !== 'undefined') (window as unknown as { __nodoDoc: Document }).__nodoDoc = doc

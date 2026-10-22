// Ponte com o backend Rust.
//
// O Nodo fala com o host por comandos Tauri. Este módulo é o único lugar que
// conhece os nomes dos comandos e o formato do snapshot, para que os
// componentes não dependam disso.

import { invoke as tauriInvoke } from '@tauri-apps/api/core'

/** Um nó achatado, com geometria absoluta, pronto para desenhar. */
export interface FlatNode {
  id: number
  text: string
  note: string
  side: 'left' | 'right' | 'center'
  depth: number
  x: number
  y: number
  width: number
  height: number
  color: string | null
  folded: boolean
  /** Etiquetas, emoji, lembrete e status do card (o núcleo omite o que está vazio). */
  card: CardFields
  child_ids: number[]
  parent: number | null
  has_children: boolean
}

/** O que o card de um nó tem além do texto e da nota: o card do Ideario sem a categoria (`nodo_core::model::CardFields`). */
export interface CardFields {
  tags?: string[]
  emoji?: string | null
  /** Data do lembrete, em milissegundos. */
  reminder_at?: number | null
  reminder_repeat?: string | null
  reminder_done?: boolean
  reminder_doing?: boolean
  /** Tem status mesmo sem lembrete. */
  status_on?: boolean
}

export interface Edge {
  from: number
  to: number
  text: string
  color: string | null
}

/** Uma nota solta no mapa (não presa a um nó), em unidades de mundo. */
export interface NoteCard {
  id: number
  x: number
  y: number
  width: number
  height: number
  text: string
}

export interface Bounds {
  min_x: number
  min_y: number
  max_x: number
  max_y: number
}

export interface Snapshot {
  nodes: FlatNode[]
  connections: Edge[]
  /** Notas soltas no mapa. */
  notes: NoteCard[]
  dirty: boolean
  path: string | null
  title: string
  can_undo: boolean
  can_redo: boolean
  /** O arquivo mudou no disco depois de lido ou salvo aqui (um agente salvou pelo MCP). */
  stale: boolean
  /** Tema do mapa (`nodo_core::MAP_STYLES`), ou null para o padrão. */
  style: string | null
  /** Estrutura (`nodo_core::STRUCTURES`): mapa, logico, organograma ou arvore; null é o mapa. */
  structure: string | null
  /** Espaçamento (`nodo_core::SPACINGS`): compacto, normal ou largo; null é o normal. */
  spacing: string | null
  bounds: Bounds
  /** Os mapas abertos, na ordem das abas. */
  tabs: TabInfo[]
  /** A aba da frente. */
  active: number | null
}

/** O que a aba mostra: o nome do mapa e se tem algo por salvar. */
export interface TabInfo {
  id: number
  title: string
  path: string | null
  dirty: boolean
}

/** Dimensões medidas no frontend para um nó. */
export interface NodeSize {
  id: number
  width: number
  height: number
}

/** Há um host Tauri do outro lado? */
export const hasHost = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window

/**
 * Modo preview: o app roda no navegador com um backend simulado.
 *
 * Serve para ajustar a interface sem compilar o Tauri, que leva minutos e não
 * deixa inspecionar nada. Ativa com `?preview` ou, em desenvolvimento, sempre
 * que não houver host.
 */
export const isPreview =
  typeof window !== 'undefined' &&
  (new URLSearchParams(location.search).has('preview') || (!hasHost && import.meta.env.DEV))

let backend: Record<string, (args: never) => unknown> | null = null

async function previewCall<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  if (!backend) {
    const mod = await import('./preview')
    mod.seedDemo()
    backend = mod.previewBackend as unknown as Record<string, (args: never) => unknown>
  }
  const fn = backend[cmd]
  if (!fn) throw new Error(`comando ${cmd} não existe no preview`)
  return fn((args ?? {}) as never) as T
}

async function call<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  if (isPreview) return previewCall<T>(cmd, args)
  if (!hasHost) throw new Error(`comando ${cmd} indisponível fora do Tauri`)
  return tauriInvoke<T>(cmd, args)
}

export const api = {
  snapshot: () => call<Snapshot>('get_snapshot'),

  newMap: (title: string) => call<Snapshot>('new_map', { title }),

  open: (path: string) => call<Snapshot>('open_map', { path }),

  /** Traz uma aba para a frente. */
  switchTab: (id: number) => call<Snapshot>('switch_tab', { id }),

  /** Fecha uma aba (a interface pergunta antes, se houver algo por salvar). */
  closeTab: (id: number) => call<Snapshot>('close_tab', { id }),

  save: (path?: string) => call<Snapshot>('save_map', { path: path ?? null }),

  addNode: (parent: number, text: string) => call<Snapshot>('add_node', { parent, text }),

  /** Filho já com nota (um card colado), num passo só do desfazer. */
  addTopic: (parent: number, text: string, note: string, color: string | null = null, card: CardFields = {}) =>
    call<Snapshot>('add_topic', { parent, text, note, color, card }),

  removeNode: (id: number) => call<Snapshot>('remove_node', { id }),

  setText: (id: number, text: string) => call<Snapshot>('set_text', { id, text }),

  setNote: (id: number, note: string) => call<Snapshot>('set_note', { id, note }),

  /** Etiquetas, emoji, lembrete e status; o núcleo limpa pelas regras do card. */
  setCard: (id: number, card: CardFields) => call<Snapshot>('set_card', { id, card }),

  /** A nota do nó como documento do editor do card (JSON do TipTap), pelas regras de Markdown do Ideario. */
  noteDoc: (id: number) => call<{ type: string; content?: unknown[] }>('note_doc', { id }),

  /** Markdown colado no card → documento do editor. */
  markdownDoc: (markdown: string) => call<{ type: string; content?: unknown[] }>('markdown_doc', { markdown }),

  /** Grava o documento do editor como a nota (Markdown) do nó. */
  setNoteDoc: (id: number, doc: unknown) => call<Snapshot>('set_note_doc', { id, doc }),

  addFreeNote: (x: number, y: number, text: string) => call<Snapshot>('add_free_note', { x, y, text }),

  setFreeNote: (id: number, text: string) => call<Snapshot>('set_free_note', { id, text }),

  placeFreeNote: (id: number, x: number, y: number, width: number, height: number) =>
    call<Snapshot>('place_free_note', { id, x, y, width, height }),

  removeFreeNote: (id: number) => call<Snapshot>('remove_free_note', { id }),

  setColor: (id: number, color: string | null) => call<Snapshot>('set_color', { id, color }),

  toggleFold: (id: number) => call<Snapshot>('toggle_fold', { id }),

  /** Sobe (`delta < 0`) ou desce o nó entre os irmãos; na raiz, só entre os ramos do mesmo lado. */
  reorder: (id: number, delta: number) => call<Snapshot>('reorder_node', { id, delta }),

  setSide: (id: number, side: 'left' | 'right') => call<Snapshot>('set_side', { id, side }),

  setMapStyle: (style: string | null) => call<Snapshot>('set_map_style', { style }),

  setStructure: (structure: string | null) => call<Snapshot>('set_structure', { structure }),

  setSpacing: (spacing: string | null) => call<Snapshot>('set_spacing', { spacing }),

  /** O mapa como texto (markdown, opml ou html), do núcleo: o mesmo que o MCP exporta. */
  exportText: (format: string) => call<string>('export_text', { format }),

  /** Grava uma exportação feita aqui no caminho escolhido (gravação atômica no Rust). */
  writeExport: (path: string, bytes: Uint8Array) => call<void>('write_export', { path, bytes: Array.from(bytes) }),

  /** Arruma tudo de novo pela estrutura, desfazendo os arrastos à mão. */
  relayout: () => call<Snapshot>('relayout'),

  reload: () => call<Snapshot>('reload_map'),

  /** Fecha sem salvar, depois de a pessoa confirmar. */
  quit: () => call<void>('quit_app'),

  undo: () => call<Snapshot>('undo'),

  redo: () => call<Snapshot>('redo'),

  search: (query: string, limit = 200) => call<number[]>('search_nodes', { query, limit }),

  /**
   * Informa ao host as dimensões medidas aqui. O Rust não mede texto: o
   * navegador já sabe fazer isso com precisão, então ele mede e o core só
   * posiciona. Ver `measure.ts`.
   */
  setSizes: (sizes: NodeSize[]) => call<Snapshot>('set_sizes', { sizes }),

  moveNode: (id: number, x: number, y: number) => call<Snapshot>('move_node', { id, x, y }),

  /** Estado do registro MCP no cliente (Claude Desktop). */
  mcpInfo: () => call<McpInfo>('mcp_info'),
  mcpInstall: () => call<McpInfo>('mcp_install'),
  mcpUninstall: () => call<McpInfo>('mcp_uninstall'),

  /** Este jeito de instalar se atualiza sozinho? (No Linux, só o AppImage; o `.deb` e o `.rpm` não.) */
  updateSupported: () => call<boolean>('update_supported'),

  /** Versão nova no último Release do GitHub, ou `null` (também quando este jeito de instalar não se atualiza). */
  checkUpdate: () => call<UpdateInfo | null>('update_check'),

  /** Baixa, instala e reinicia; `onProgress` recebe bytes baixados e total (0 quando não se sabe). */
  async installUpdate(onProgress: (done: number, total: number) => void): Promise<void> {
    if (isPreview) return call<void>('update_install')
    const { listen } = await import('@tauri-apps/api/event')
    const stop = await listen<[number, number]>('update-progress', (e) => onProgress(e.payload[0], e.payload[1]))
    try {
      await call<void>('update_install')
    } finally {
      stop()
    }
  },
}

/** Uma versão nova encontrada pelo atualizador. */
export interface UpdateInfo {
  version: string
  /** Texto do Release (o que mudou), se houver. */
  notes: string | null
}

/** O que a página de MCP mostra. */
export interface McpInfo {
  command: string
  args: string[]
  claudeCode: string
  desktopConfig: string | null
  desktopFound: boolean
  desktopInstalled: boolean
}

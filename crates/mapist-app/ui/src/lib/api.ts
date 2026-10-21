// Ponte com o backend Rust.
//
// O Mapist fala com o host por comandos Tauri. Este módulo é o único lugar que
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
  child_ids: number[]
  parent: number | null
  has_children: boolean
}

export interface Edge {
  from: number
  to: number
  text: string
  color: string | null
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
  dirty: boolean
  path: string | null
  title: string
  can_undo: boolean
  can_redo: boolean
  bounds: Bounds
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

  save: (path?: string) => call<Snapshot>('save_map', { path: path ?? null }),

  addNode: (parent: number, text: string) => call<Snapshot>('add_node', { parent, text }),

  removeNode: (id: number) => call<Snapshot>('remove_node', { id }),

  setText: (id: number, text: string) => call<Snapshot>('set_text', { id, text }),

  setNote: (id: number, note: string) => call<Snapshot>('set_note', { id, note }),

  setColor: (id: number, color: string | null) => call<Snapshot>('set_color', { id, color }),

  toggleFold: (id: number) => call<Snapshot>('toggle_fold', { id }),

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

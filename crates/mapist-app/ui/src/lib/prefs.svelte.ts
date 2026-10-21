// Preferências da interface que sobrevivem entre sessões.
//
// Só o que é do aparelho mora aqui: zoom, painel, busca e traçado das linhas. O documento em si
// nunca passa por isto — ele é do Rust.

import type { LinkStyle } from './render'

const PREFIX = 'mapist.ui.'

function read<T>(key: string, fallback: T): T {
  try {
    const raw = localStorage.getItem(PREFIX + key)
    if (raw === null) return fallback
    return JSON.parse(raw) as T
  } catch {
    return fallback
  }
}

function write(key: string, value: unknown) {
  try {
    localStorage.setItem(PREFIX + key, JSON.stringify(value))
  } catch {
    // sem armazenamento: vale só nesta sessão
  }
}

class Prefs {
  /** Zoom da última sessão, para reabrir na mesma escala. */
  zoom = $state(read('zoom', 1))
  /** Painel lateral visível. Na primeira vez, aberto só quando cabe ao lado do mapa (no celular ele cobre metade). */
  inspectorOpen = $state(read('inspector', typeof matchMedia !== 'function' || matchMedia('(min-width: 900px)').matches))
  /** Última busca usada, restaurada no campo. */
  search = $state(read('search', ''))
  /** Traçado das linhas entre os nós. */
  linkStyle = $state<LinkStyle>(read<LinkStyle>('linkStyle', 'curved') === 'orthogonal' ? 'orthogonal' : 'curved')

  setZoom(z: number) {
    this.zoom = z
    write('zoom', z)
  }

  toggleInspector() {
    this.inspectorOpen = !this.inspectorOpen
    write('inspector', this.inspectorOpen)
  }

  setLinkStyle(style: LinkStyle) {
    this.linkStyle = style
    write('linkStyle', style)
  }

  setSearch(q: string) {
    this.search = q
    write('search', q)
  }
}

export const prefs = new Prefs()

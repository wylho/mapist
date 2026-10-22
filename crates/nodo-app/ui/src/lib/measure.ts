// Medição de texto no frontend.
//
// O core Rust estima larguras por contagem de caracteres, o que erra com
// fontes proporcionais, negrito e emoji. O navegador mede de verdade, então
// medimos aqui e enviamos as dimensões ao host.
//
// Uma única canvas 2D fora de tela é reutilizada: criar uma por medição seria
// caro e não traria benefício.

import type { CardFields, CardView, FlatNode } from './api'

import { fontStack, levelFont } from './font'
import { levelStyle, mapTheme, type MapTheme } from './maptheme'
import { flushSync, mount, unmount } from 'svelte'
import MapCard from '../components/MapCard.svelte'
import { CARD, isCard } from './cardtext'

/** Altura de linha, proporcional ao tamanho do texto do nível. */
const LINE_RATIO = 1.4

let ctx: CanvasRenderingContext2D | null = null

function context(): CanvasRenderingContext2D {
  if (!ctx) {
    const c = document.createElement('canvas')
    const got = c.getContext('2d')
    if (!got) throw new Error('canvas 2D indisponível para medir texto')
    ctx = got
  }
  return ctx
}

/**
 * Mede um nó, em unidades de mundo.
 *
 * Sem parâmetro de zoom de propósito: a geometria do mapa não pode depender de
 * quanto ele está ampliado na tela. O desenho escala tudo junto com
 * `ctx.scale()`, então medir e desenhar usam sempre a mesma referência.
 */
export function measureNode(node: FlatNode, theme: MapTheme = mapTheme(null)): { width: number; height: number } {
  if (isCard(node)) return measureCard(node.text, node.view, node.card)
  const c = context()
  const style = levelStyle(theme, node.depth)
  c.font = levelFont(style)

  const lines = node.text.split('\n')
  let widest = 0
  for (const line of lines) {
    const w = c.measureText(line).width
    if (w > widest) widest = w
  }

  return {
    width: Math.max(style.minW, widest + style.padX),
    height: Math.max(style.minH, lines.length * style.size * LINE_RATIO + style.padY),
  }
}

/** Mede todos os nós de um snapshot, em unidades de mundo, no tema do mapa. */
export function measureAll(nodes: FlatNode[], theme: MapTheme = mapTheme(null)): { id: number; width: number; height: number }[] {
  return nodes.map((n) => ({ id: n.id, ...measureNode(n, theme) }))
}

/** Trunca um texto para caber em `maxWidth`, com reticências, na fonte do nível. Tudo em unidades de mundo. */
export function fitText(text: string, maxWidth: number, style: { size: number; weight: number }): string {
  const c = context()
  c.font = levelFont(style)
  if (c.measureText(text).width <= maxWidth) return text
  let out = text
  while (out.length > 1 && c.measureText(out + '…').width > maxWidth) {
    out = out.slice(0, -1)
  }
  return out + '…'
}

let host: HTMLElement | null = null
const cardSizes = new Map<string, { width: number; height: number }>()

/**
 * Mede o card de um nó com nota no próprio DOM, com o mesmo componente e o mesmo CSS da tela (`CardBody` do Atrio).
 * Fora do zoom: o card na tela é escalado por `transform`, então a medida aqui já é em unidades de mundo.
 */
export function measureCard(title: string, view: CardView | undefined, card: CardFields = {}): { width: number; height: number } {
  const key = `${title}\u0000${JSON.stringify(view ?? null)}\u0000${JSON.stringify(card)}`
  const hit = cardSizes.get(key)
  if (hit) return hit
  if (!host) {
    host = document.createElement('div')
    host.setAttribute('aria-hidden', 'true')
    host.style.cssText = `position:absolute;left:-10000px;top:0;visibility:hidden;pointer-events:none;width:${CARD.width}px`
    document.body.appendChild(host)
  }
  host.innerHTML = '<div class="card"></div>'
  const el = host.firstElementChild as HTMLElement
  const app = mount(MapCard, { target: el, props: { title, view, card, onstatus: () => {} } })
  flushSync()
  const size = { width: CARD.width, height: Math.ceil(el.getBoundingClientRect().height) }
  unmount(app)
  if (cardSizes.size > 500) cardSizes.clear()
  cardSizes.set(key, size)
  return size
}

/** Esquece as medidas dos cards (a fonte ou o tema mudou). */
export function resetCardSizes() {
  cardSizes.clear()
}

/** Largura de um trecho numa fonte do card (o desenho para exportar usa, inclusive o SVG, que não sabe medir). */
export function measureRun(text: string, size: number, weight: number, mono = false): number {
  const c = context()
  c.font = `${weight} ${size}px ${mono ? 'ui-monospace, monospace' : fontStack()}`
  return c.measureText(text).width
}

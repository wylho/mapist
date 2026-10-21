// Medição de texto no frontend.
//
// O core Rust estima larguras por contagem de caracteres, o que erra com
// fontes proporcionais, negrito e emoji. O navegador mede de verdade, então
// medimos aqui e enviamos as dimensões ao host.
//
// Uma única canvas 2D fora de tela é reutilizada: criar uma por medição seria
// caro e não traria benefício.

import type { FlatNode } from './api'

const PAD_X = 36 // padding horizontal + folga da borda
const PAD_Y = 22 // padding vertical
const MIN_W = 84
const MIN_H = 44
const LINE_H = 19

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

/** Família e tamanho da fonte usados para desenhar os nós. */
function fontFor(isRoot: boolean, zoom: number): string {
  const family =
    getComputedStyle(document.documentElement).getPropertyValue('--font-body').trim() ||
    'system-ui, sans-serif'
  const size = (isRoot ? 14 : 13.5) * Math.max(0.5, Math.min(zoom, 3))
  return `${isRoot ? 700 : 600} ${size}px ${family}`
}

/**
 * Mede um nó e devolve suas dimensões em unidades de mapa.
 *
 * `zoom` entra porque a fonte desenhada acompanha o zoom; medir sempre em 1.0
 * daria caixas apertadas quando ampliado.
 */
export function measureNode(node: FlatNode, zoom = 1): { width: number; height: number } {
  const c = context()
  const isRoot = node.parent === null
  c.font = fontFor(isRoot, zoom)

  const lines = node.text.split('\n')
  let widest = 0
  for (const line of lines) {
    const w = c.measureText(line).width
    if (w > widest) widest = w
  }

  return {
    width: Math.max(MIN_W, widest + PAD_X),
    height: Math.max(MIN_H, lines.length * LINE_H + PAD_Y),
  }
}

/** Mede todos os nós de um snapshot. */
export function measureAll(nodes: FlatNode[], zoom = 1): { id: number; width: number; height: number }[] {
  return nodes.map((n) => ({ id: n.id, ...measureNode(n, zoom) }))
}

/** Trunca um texto para caber em `maxWidth`, com reticências. */
export function fitText(text: string, maxWidth: number, isRoot: boolean, zoom: number): string {
  const c = context()
  c.font = fontFor(isRoot, zoom)
  if (c.measureText(text).width <= maxWidth) return text
  let out = text
  while (out.length > 1 && c.measureText(out + '…').width > maxWidth) {
    out = out.slice(0, -1)
  }
  return out + '…'
}

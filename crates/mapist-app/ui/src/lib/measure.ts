// Medição de texto no frontend.
//
// O core Rust estima larguras por contagem de caracteres, o que erra com
// fontes proporcionais, negrito e emoji. O navegador mede de verdade, então
// medimos aqui e enviamos as dimensões ao host.
//
// Uma única canvas 2D fora de tela é reutilizada: criar uma por medição seria
// caro e não traria benefício.

import type { FlatNode } from './api'

import { nodeFont } from './font'

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

/**
 * Mede um nó, em unidades de mundo.
 *
 * Sem parâmetro de zoom de propósito: a geometria do mapa não pode depender de
 * quanto ele está ampliado na tela. O desenho escala tudo junto com
 * `ctx.scale()`, então medir e desenhar usam sempre a mesma referência.
 */
export function measureNode(node: FlatNode): { width: number; height: number } {
  const c = context()
  const isRoot = node.parent === null
  c.font = nodeFont(isRoot)

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

/** Mede todos os nós de um snapshot, em unidades de mundo. */
export function measureAll(nodes: FlatNode[]): { id: number; width: number; height: number }[] {
  return nodes.map((n) => ({ id: n.id, ...measureNode(n) }))
}

/** Trunca um texto para caber em `maxWidth`, com reticências. Tudo em unidades de mundo. */
export function fitText(text: string, maxWidth: number, isRoot: boolean): string {
  const c = context()
  c.font = nodeFont(isRoot)
  if (c.measureText(text).width <= maxWidth) return text
  let out = text
  while (out.length > 1 && c.measureText(out + '…').width > maxWidth) {
    out = out.slice(0, -1)
  }
  return out + '…'
}

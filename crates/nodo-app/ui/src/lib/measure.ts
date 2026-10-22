// Medição de texto no frontend.
//
// O core Rust estima larguras por contagem de caracteres, o que erra com
// fontes proporcionais, negrito e emoji. O navegador mede de verdade, então
// medimos aqui e enviamos as dimensões ao host.
//
// Uma única canvas 2D fora de tela é reutilizada: criar uma por medição seria
// caro e não traria benefício.

import type { CardFields, CardView, FlatNode } from './api'

import { levelFont } from './font'
import { levelStyle, mapTheme, type MapTheme } from './maptheme'
import { flushSync, mount, unmount } from 'svelte'
import { NoteCard } from '@wylho/atrio'
import { CARD, isCard, summaryOf } from './cardtext'

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
export function measureNode(node: FlatNode, theme: MapTheme = mapTheme(null), titleOnly = false): { width: number; height: number } {
  if (!titleOnly && isCard(node)) return measureCard(node.text, node.view, node.card)
  const c = context()
  const style = levelStyle(theme, node.depth)
  c.font = levelFont(style)

  // O texto quebra em linhas na largura máxima do nível, em vez de esticar a caixa (pedido do usuário).
  const lines = wrapText(node.text, maxBoxWidth(node.depth) - style.padX, style)
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

/** Largura máxima da caixa de um tópico: a mesma do card (`CARD.width`), e um pouco mais na raiz. */
export function maxBoxWidth(depth: number): number {
  // PRÉVIA DE OPÇÕES (provisório): `?maxw=1|2|3` = vezes a largura do card.
  const k = Number(typeof location !== 'undefined' ? new URLSearchParams(location.search).get('maxw') ?? 1 : 1) || 1
  return CARD.width * k
}

/**
 * Quebra o texto em linhas que cabem em `maxWidth` (unidades de mundo), nas palavras; uma palavra maior que a linha
 * é cortada nas letras. Quebras de linha do próprio texto ficam. A mesma conta mede (`measureNode`) e desenha.
 */
export function wrapText(text: string, maxWidth: number, style: { size: number; weight: number }): string[] {
  const c = context()
  c.font = levelFont(style)
  const fits = (s: string) => c.measureText(s).width <= maxWidth
  const out: string[] = []
  for (const para of text.split('\n')) {
    let line = ''
    for (const word of para.split(/\s+/).filter(Boolean)) {
      const next = line ? `${line} ${word}` : word
      if (fits(next)) {
        line = next
        continue
      }
      if (line) out.push(line)
      // Palavra sozinha maior que a linha (um endereço, por exemplo): vai em pedaços.
      line = ''
      for (const ch of word) {
        if (line && !fits(line + ch)) {
          out.push(line)
          line = ''
        }
        line += ch
      }
    }
    out.push(line)
  }
  return out
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
  host.innerHTML = ''
  const app = mount(NoteCard, { target: host, props: { card: summaryOf({ text: title, card, view }, 'none'), thumb: () => '', onstatus: () => {} } })
  const el = host.firstElementChild as HTMLElement
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


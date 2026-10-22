// O card exportado é o próprio card da tela: o `NoteCard` do Atrio é montado fora da vista, no tamanho real, e o que
// o navegador desenhou vira uma lista de operações (fundos e bordas, linhas de texto, fotos, ícones) que `render.ts`
// repete no canvas da exportação ou no SVG. Assim PNG, SVG e PDF mostram exatamente o card da tela (emoji, capa,
// pílulas, status, etiquetas), sem um segundo desenho à mão que pudesse divergir.

import { flushSync, mount, unmount } from 'svelte'
import { NoteCard, nearestNoteColor, type NoteColor } from '@wylho/atrio'
import { attachments, type Snapshot } from './api'
import { isCard, summaryOf } from './cardtext'

/** Raios dos quatro cantos: cima-esquerda, cima-direita, baixo-direita, baixo-esquerda. */
export type Radii = [number, number, number, number]

export type Op =
  | { t: 'box'; x: number; y: number; w: number; h: number; r: Radii; fill: string | null; stroke: string | null; sw: number; alpha: number }
  | { t: 'text'; x: number; y: number; text: string; font: string; color: string; alpha: number; strike: number | null; width: number }
  | { t: 'img'; x: number; y: number; w: number; h: number; img: HTMLImageElement; clip: { x: number; y: number; w: number; h: number; r: Radii } | null; alpha: number }
  | { t: 'svg'; x: number; y: number; w: number; h: number; markup: string; img: HTMLImageElement; alpha: number }

/** O card pronto para repetir: as operações, relativas ao canto do card. */
export interface CardPaint {
  width: number
  height: number
  ops: Op[]
}

/** Cor sem tinta: vazia, `transparent` ou com opacidade zero (`rgba(…, 0)`; um `rgb(232, 164, 0)` não é). */
export const clear = (c: string) => !c || c === 'transparent' || /^rgba\((?:[^,]+,){3}\s*0(?:\.0*)?\)$/.test(c.trim())

/** Qualquer cor do CSS (oklch, color-mix, color(srgb …)) como `rgba()`, que o SVG e todo programa entendem. */
const rgbCache = new Map<string, string>()
let pixel: CanvasRenderingContext2D | null = null
function rgb(c: string): string {
  const hit = rgbCache.get(c)
  if (hit) return hit
  pixel ??= document.createElement('canvas').getContext('2d', { willReadFrequently: true })
  if (!pixel) return c
  pixel.clearRect(0, 0, 1, 1)
  pixel.fillStyle = c
  pixel.fillRect(0, 0, 1, 1)
  const [r, g, b, a] = pixel.getImageData(0, 0, 1, 1).data
  const out = a === 255 ? `rgb(${r}, ${g}, ${b})` : `rgba(${r}, ${g}, ${b}, ${Math.round((a / 255) * 1000) / 1000})`
  rgbCache.set(c, out)
  return out
}

let measurer: CanvasRenderingContext2D | null = null
function metrics(font: string, text: string) {
  measurer ??= document.createElement('canvas').getContext('2d')
  if (!measurer) return { ascent: 0, descent: 0, width: 0 }
  measurer.font = font
  const m = measurer.measureText(text)
  return { ascent: m.fontBoundingBoxAscent, descent: m.fontBoundingBoxDescent, width: m.width }
}

function radiiOf(cs: CSSStyleDeclaration): Radii {
  return [cs.borderTopLeftRadius, cs.borderTopRightRadius, cs.borderBottomRightRadius, cs.borderBottomLeftRadius].map(
    (v) => parseFloat(v) || 0,
  ) as Radii
}

function fontOf(cs: CSSStyleDeclaration): string {
  return `${cs.fontStyle === 'italic' ? 'italic ' : ''}${cs.fontWeight} ${cs.fontSize} ${cs.fontFamily}`
}

/** Carrega uma imagem num endereço que não "suja" o canvas (CORS) e devolve já decodificada, ou null. */
async function load(src: string): Promise<HTMLImageElement | null> {
  const img = new Image()
  img.crossOrigin = 'anonymous'
  img.src = src
  try {
    await img.decode()
    return img
  } catch {
    return null
  }
}

/** A foto como `data:` no tamanho em que aparece (2× para ficar nítida): serve ao canvas e vai dentro do SVG. */
async function inline(src: string, w: number, h: number): Promise<HTMLImageElement | null> {
  const img = await load(src)
  if (!img) return null
  const c = document.createElement('canvas')
  c.width = Math.max(1, Math.round(w * 2))
  c.height = Math.max(1, Math.round(h * 2))
  const ctx = c.getContext('2d')
  if (!ctx) return null
  // object-fit: cover, como o CSS do card.
  const k = Math.max(c.width / img.naturalWidth, c.height / img.naturalHeight)
  const dw = img.naturalWidth * k
  const dh = img.naturalHeight * k
  ctx.drawImage(img, (c.width - dw) / 2, (c.height - dh) / 2, dw, dh)
  try {
    return await load(c.toDataURL('image/webp', 0.85))
  } catch {
    return null
  }
}

/** Lê o card montado e devolve as operações. */
async function read(root: HTMLElement): Promise<CardPaint> {
  const base = root.getBoundingClientRect()
  const ops: Op[] = []
  const pending: Promise<void>[] = []
  const rel = (r: DOMRect) => ({ x: r.left - base.left, y: r.top - base.top, w: r.width, h: r.height })

  function strikeOf(el: Element): boolean {
    for (let e: Element | null = el; e && e !== root.parentElement; e = e.parentElement) {
      if (getComputedStyle(e).textDecorationLine.includes('line-through')) return true
    }
    return false
  }

  function text(node: Text, cs: CSSStyleDeclaration, alpha: number) {
    const value = node.data
    if (!value.trim()) return
    const font = fontOf(cs)
    const strike = strikeOf(node.parentElement!)
    const range = document.createRange()
    // Palavra por palavra, juntando as que caem na mesma linha.
    const lines: { top: number; left: number; height: number; text: string }[] = []
    for (const m of value.matchAll(/\S+\s*/g)) {
      range.setStart(node, m.index)
      range.setEnd(node, m.index + m[0].trimEnd().length)
      const r = range.getClientRects()[0]
      if (!r || r.width === 0) continue
      const last = lines.at(-1)
      if (last && Math.abs(last.top - r.top) < 2) last.text += m[0]
      else lines.push({ top: r.top, left: r.left, height: r.height, text: m[0] })
    }
    for (const l of lines) {
      const t = l.text.trimEnd()
      const k = metrics(font, t)
      const y = l.top - base.top + (l.height - (k.ascent + k.descent)) / 2 + k.ascent
      ops.push({
        t: 'text', x: l.left - base.left, y, text: t, font, color: rgb(cs.color), alpha, width: k.width,
        strike: strike ? l.top - base.top + l.height * 0.55 : null,
      })
    }
  }

  function walk(el: Element, parentAlpha: number, clip: Extract<Op, { t: 'img' }>['clip']) {
    const cs = getComputedStyle(el)
    if (cs.display === 'none') return
    const alpha = parentAlpha * (parseFloat(cs.opacity) || 0)
    if (alpha <= 0.01) return
    const box = rel(el.getBoundingClientRect())
    const sw = parseFloat(cs.borderTopWidth) || 0
    const fill = clear(cs.backgroundColor) ? null : rgb(cs.backgroundColor)
    const stroke = sw > 0 && !clear(cs.borderTopColor) ? rgb(cs.borderTopColor) : null
    if (fill || stroke) ops.push({ t: 'box', ...box, r: radiiOf(cs), fill, stroke, sw, alpha })
    if (cs.overflow === 'hidden' && radiiOf(cs).some((v) => v > 0)) clip = { ...box, r: radiiOf(cs) }

    if (el instanceof HTMLImageElement) {
      const src = el.currentSrc || el.src
      const op = { t: 'img' as const, ...box, img: new Image(), clip, alpha }
      ops.push(op)
      pending.push(inline(src, box.w, box.h).then((img) => void (img ? (op.img = img) : ops.splice(ops.indexOf(op), 1))))
      return
    }
    if (el instanceof SVGSVGElement) {
      // Ícones (Lucide): o próprio SVG, com a cor do texto em vez de `currentColor`.
      const copy = el.cloneNode(true) as SVGSVGElement
      copy.setAttribute('xmlns', 'http://www.w3.org/2000/svg')
      copy.setAttribute('width', String(box.w))
      copy.setAttribute('height', String(box.h))
      const markup = copy.outerHTML.replaceAll('currentColor', rgb(cs.color))
      const op = { t: 'svg' as const, ...box, markup, img: new Image(), alpha }
      ops.push(op)
      pending.push(load(`data:image/svg+xml;charset=utf-8,${encodeURIComponent(markup)}`).then((img) => void (img && (op.img = img))))
      return
    }
    // Marcador da lista (`::before` com `attr(data-mark)`), na primeira linha do item.
    const before = getComputedStyle(el, '::before')
    if (before.content && before.content !== 'none' && before.content !== 'normal' && el.getAttribute('data-mark')) {
      const mark = el.getAttribute('data-mark')!
      const font = fontOf(before)
      const k = metrics(font, mark)
      const line = parseFloat(cs.lineHeight) || parseFloat(cs.fontSize) * 1.4
      ops.push({
        t: 'text', x: box.x + (parseFloat(before.left) || 0), y: box.y + (line - (k.ascent + k.descent)) / 2 + k.ascent,
        text: mark, font, color: rgb(before.color), alpha, strike: null, width: k.width,
      })
    }
    // Bolinha desenhada só pelo CSS (`::after` vazio, centrado): as cores do seletor de status.
    const after = getComputedStyle(el, '::after')
    if (after.content === '""' && after.position !== 'absolute') {
      const w = parseFloat(after.width) || 0
      const h = parseFloat(after.height) || 0
      if (w > 0 && h > 0) {
        const k = /matrix\(([-\d.]+)/.exec(after.transform)?.[1]
        const scale = k ? parseFloat(k) : 1
        const cw = w * scale
        const ch = h * scale
        const cx = box.x + (box.w - cw) / 2
        const cy = box.y + (box.h - ch) / 2
        const a2 = alpha * (parseFloat(after.opacity) || 0)
        const r = radiiOf(after).map((v) => v * scale) as Radii
        // Anel do `box-shadow` (sem desfoque), por baixo.
        const ring = /^(.+?\))\s+0px\s+0px\s+0px\s+([\d.]+)px/.exec(after.boxShadow)
        if (ring) {
          const sp = parseFloat(ring[2])
          ops.push({ t: 'box', x: cx - sp, y: cy - sp, w: cw + sp * 2, h: ch + sp * 2, r: r.map((v) => v + sp) as Radii, fill: rgb(ring[1]), stroke: null, sw: 0, alpha: a2 })
        }
        const sw = parseFloat(after.borderTopWidth) || 0
        ops.push({
          t: 'box', x: cx, y: cy, w: cw, h: ch, r,
          fill: clear(after.backgroundColor) ? null : rgb(after.backgroundColor),
          stroke: sw > 0 && !clear(after.borderTopColor) ? rgb(after.borderTopColor) : null,
          sw: sw * scale, alpha: a2,
        })
      }
    }
    for (const child of el.childNodes) {
      if (child.nodeType === Node.TEXT_NODE) text(child as Text, cs, alpha)
      else if (child instanceof Element) walk(child, alpha, clip)
    }
  }

  walk(root, 1, null)
  await Promise.all(pending)
  return { width: base.width, height: base.height, ops }
}

/** Monta o card de um resumo fora da vista, lê e desmonta. */
async function paintOf(props: Record<string, unknown>, width: number): Promise<CardPaint> {
  const host = document.createElement('div')
  host.setAttribute('aria-hidden', 'true')
  host.style.cssText = `position:absolute;left:-10000px;top:0;pointer-events:none;width:${width}px`
  document.body.appendChild(host)
  const app = mount(NoteCard, { target: host, props: props as never })
  flushSync()
  const root = host.firstElementChild as HTMLElement
  root.style.position = 'relative'
  root.style.left = '0'
  root.style.top = '0'
  root.style.transform = 'none'
  // As fotos da capa: esperar chegarem, senão o card sai sem elas.
  await Promise.all([...root.querySelectorAll('img')].map((i) => i.decode().catch(() => {})))
  try {
    return await read(root)
  } finally {
    unmount(app)
    host.remove()
  }
}

/** Chave de um card no mapa: `n:<id>` para um tópico, `f:<id>` para uma nota solta. */
export const cardKey = (kind: 'n' | 'f', id: number) => `${kind}:${id}`

/**
 * Lê todos os cards do mapa (tópicos que são card e notas soltas), com as cores que a tela usa. `branch` dá a cor do
 * ramo de cada tópico (a do card é a cor de nota do Atrio mais próxima, como na tela).
 */
export async function paintCards(s: Snapshot, branch: Map<number, string>): Promise<Map<string, CardPaint>> {
  const out = new Map<string, CardPaint>()
  const base = { thumb: (h: string) => attachments.thumb(h), onstatus: () => {}, class: 'map-card', tabindex: -1 }
  for (const n of s.nodes) {
    if (!isCard(n)) continue
    const card = summaryOf(n, nearestNoteColor(branch.get(n.id) ?? null))
    out.set(cardKey('n', n.id), await paintOf({ ...base, card }, n.width))
  }
  for (const f of s.notes) {
    const card = summaryOf({ text: f.title, card: f.card, view: f.view }, (f.color ?? 'none') as NoteColor)
    out.set(cardKey('f', f.id), await paintOf({ ...base, card, class: 'map-card free-card' }, f.width))
  }
  return out
}

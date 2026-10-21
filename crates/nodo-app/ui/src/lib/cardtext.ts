// Nó com nota desenhado como o card de nota do Ideario: título em cima e, embaixo, uma prévia da nota com a estrutura
// do Markdown (parágrafos, listas, tarefas, títulos, negrito). Nó sem nota continua sendo a pílula de sempre.
//
// Tudo em unidades de mundo (a regra do zoom vale aqui também): `layoutCard` quebra as linhas com uma função de medir
// e devolve posições prontas; a medição (`measure.ts`) usa a altura, o desenho (`render.ts`) usa as linhas. Como as
// duas leem o mesmo resultado, o que se mede é exatamente o que se desenha.

/** Medidas do card, copiadas do `.card` do Ideario (raio 16, corpo 12/13, título 15.5, texto 13.5/1.45). */
export const CARD = {
  width: 240,
  radius: 16,
  padX: 13,
  padY: 12,
  gap: 6,
  titleSize: 15.5,
  titleLine: 1.25,
  bodySize: 13.5,
  bodyLine: 1.45,
  /** A prévia mostra até tantas linhas; o resto esmaece, como a prévia do Ideario. */
  maxLines: 9,
}

export interface Run {
  text: string
  bold: boolean
}

export type BlockKind = 'p' | 'h' | 'li' | 'ol' | 'task' | 'code' | 'quote'

export interface Block {
  kind: BlockKind
  runs: Run[]
  depth: number
  done?: boolean
  /** Marcador da lista numerada ("1."). */
  mark?: string
}

/** Uma linha pronta para desenhar, relativa ao canto do card. */
export interface CardLine {
  kind: BlockKind | 'title'
  x: number
  /** Linha de base. */
  y: number
  runs: { text: string; bold: boolean; x: number }[]
  /** Primeira linha do bloco: leva o marcador (•, 1., caixa da tarefa). */
  first: boolean
  done?: boolean
  mark?: string
  /** Onde fica o marcador, à esquerda do texto. */
  markX: number
  /** Esmaecida no fim da prévia. */
  alpha: number
}

export interface CardLayout {
  width: number
  height: number
  lines: CardLine[]
}

/** Uma nota pede card quando tem algum texto. */
export function isCard(note: string, depth: number): boolean {
  return depth > 0 && note.trim().length > 0
}

/** Tira a marcação de linha do Markdown e devolve os trechos, com o negrito separado. */
export function inline(text: string): Run[] {
  const plain = text
    .replace(/!\[([^\]]*)\]\([^)]*\)/g, '$1')
    .replace(/\[([^\]]+)\]\([^)]*\)/g, '$1')
    .replace(/`([^`]+)`/g, '$1')
    .replace(/~~([^~]+)~~/g, '$1')
  const runs: Run[] = []
  const re = /(\*\*|__)(.+?)\1/g
  let at = 0
  for (let m = re.exec(plain); m; m = re.exec(plain)) {
    if (m.index > at) runs.push({ text: plain.slice(at, m.index), bold: false })
    runs.push({ text: m[2], bold: true })
    at = m.index + m[0].length
  }
  if (at < plain.length) runs.push({ text: plain.slice(at), bold: false })
  // Itálico vira texto comum (o card não tem itálico), sem os asteriscos.
  return runs
    .map((r) => ({ ...r, text: r.text.replace(/(^|[^*\w])[*_]([^*_]+)[*_](?=$|[^*\w])/g, '$1$2') }))
    .filter((r) => r.text.length > 0)
}

/** A nota em blocos, uma linha do texto por bloco (as quebras de linha da nota valem, como no balão). */
export function parseNote(md: string): Block[] {
  const out: Block[] = []
  let fence = false
  for (const raw of md.replace(/\r/g, '').split('\n')) {
    if (/^\s*(```|~~~)/.test(raw)) {
      fence = !fence
      continue
    }
    if (fence) {
      if (raw.trim()) out.push({ kind: 'code', runs: [{ text: raw, bold: false }], depth: 0 })
      continue
    }
    if (!raw.trim() || /^\s*([-*_])(\s*\1){2,}\s*$/.test(raw)) continue
    const depth = Math.min(3, Math.floor((raw.match(/^\s*/)?.[0].replace(/\t/g, '  ').length ?? 0) / 2))
    const line = raw.trim()
    let m: RegExpMatchArray | null
    if ((m = line.match(/^#{1,6}\s+(.*)$/))) out.push({ kind: 'h', runs: inline(m[1]), depth: 0 })
    else if ((m = line.match(/^[-*+]\s+\[([ xX])\]\s*(.*)$/)))
      out.push({ kind: 'task', runs: inline(m[2]), depth, done: m[1] !== ' ' })
    else if ((m = line.match(/^[-*+]\s+(.*)$/))) out.push({ kind: 'li', runs: inline(m[1]), depth })
    else if ((m = line.match(/^(\d+)[.)]\s+(.*)$/))) out.push({ kind: 'ol', runs: inline(m[2]), depth, mark: `${m[1]}.` })
    else if ((m = line.match(/^>\s?(.*)$/))) out.push({ kind: 'quote', runs: inline(m[1]), depth: 0 })
    else out.push({ kind: 'p', runs: inline(line), depth: 0 })
  }
  return out.filter((b) => b.runs.length > 0 || b.kind === 'task')
}

/** Mede um trecho na fonte pedida. */
export type Measure = (text: string, size: number, weight: number, mono?: boolean) => number

/** Quebra trechos em linhas de até `max` de largura, palavra por palavra (uma palavra maior que a linha é cortada). */
function wrap(runs: Run[], max: number, size: number, weight: (bold: boolean) => number, measure: Measure, mono = false) {
  const lines: { text: string; bold: boolean; x: number }[][] = [[]]
  let x = 0
  const words = runs.flatMap((r) => r.text.split(/(\s+)/).filter(Boolean).map((t) => ({ text: t, bold: r.bold })))
  for (const w of words) {
    const space = /^\s+$/.test(w.text)
    if (space && x === 0) continue
    let width = measure(w.text, size, weight(w.bold), mono)
    if (!space && x > 0 && x + width > max) {
      lines.push([])
      x = 0
    }
    let text = w.text
    while (!space && width > max && text.length > 1) {
      text = text.slice(0, -1)
      width = measure(text + '…', size, weight(w.bold), mono)
    }
    if (text !== w.text) text += '…'
    const cur = lines[lines.length - 1]
    const last = cur[cur.length - 1]
    if (last && last.bold === w.bold) last.text += text
    else cur.push({ text, bold: w.bold, x })
    x += width
  }
  return lines
}

/** Posiciona título e prévia dentro do card. A altura cobre exatamente o que se desenha. */
export function layoutCard(title: string, note: string, measure: Measure): CardLayout {
  const inner = CARD.width - CARD.padX * 2
  const lines: CardLine[] = []
  let y = CARD.padY
  const tLine = CARD.titleSize * CARD.titleLine
  for (const runs of wrap([{ text: title || ' ', bold: true }], inner, CARD.titleSize, () => 600, measure)) {
    lines.push({ kind: 'title', x: CARD.padX, y: y + CARD.titleSize, runs, first: true, markX: CARD.padX, alpha: 1 })
    y += tLine
  }
  y += CARD.gap
  const bLine = CARD.bodySize * CARD.bodyLine
  let count = 0
  let clipped = false
  for (const b of parseNote(note)) {
    if (count >= CARD.maxLines) {
      clipped = true
      break
    }
    const indent = b.depth * 12 + (b.kind === 'li' ? 14 : b.kind === 'ol' ? 18 : b.kind === 'task' ? 22 : b.kind === 'quote' ? 10 : 0)
    const size = b.kind === 'code' ? 11.5 : CARD.bodySize
    const weight = (bold: boolean) => (b.kind === 'h' ? 650 : bold ? 600 : 400)
    const wrapped = wrap(b.runs.length ? b.runs : [{ text: ' ', bold: false }], inner - indent, size, weight, measure, b.kind === 'code')
    if (b.kind === 'h' && count > 0) y += 4
    wrapped.forEach((runs, i) => {
      if (count >= CARD.maxLines) {
        clipped = true
        return
      }
      lines.push({
        kind: b.kind,
        x: CARD.padX + indent,
        y: y + size + (bLine - size) / 2 - 1,
        runs,
        first: i === 0,
        done: b.done,
        mark: b.kind === 'li' ? '•' : b.mark,
        markX: CARD.padX + b.depth * 12,
        alpha: 1,
      })
      y += bLine
      count++
    })
  }
  // Como a prévia do Ideario: com mais do que cabe, as duas últimas linhas esmaecem.
  if (clipped) {
    const body = lines.filter((l) => l.kind !== 'title')
    if (body.length) body[body.length - 1].alpha = 0.25
    if (body.length > 1) body[body.length - 2].alpha = 0.6
  }
  return { width: CARD.width, height: Math.ceil(y + CARD.padY), lines }
}

// ── o card de verdade, em HTML ──────────────────────────────────────────────
// Na tela, o nó com nota é o próprio card do Ideario: a mesma marcação (`.card > .card-body > h3 + .preview` com os
// blocos `pv-*`) e o mesmo CSS (`ideario.css`, cópia literal). O canvas só desenha os ramos e a seleção por baixo.
// `layoutCard` acima continua servindo à exportação, que é desenhada em canvas/SVG.

/** Limites da prévia, os mesmos de `preview_of` no Ideario (o card corta pela altura). */
const MAX_BLOCKS = 24
const MAX_TASKS = 12
const MAX_CHARS = 1200

const esc = (s: string) => s.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;').replace(/"/g, '&quot;')
const runsHtml = (runs: Run[]) => runs.map((r) => (r.bold ? `<b>${esc(r.text)}</b>` : esc(r.text))).join('')
// O check da caixa de tarefa feita: o ícone Check do Lucide (10 px, traço 3), como no card do Ideario.
const CHECK =
  '<svg xmlns="http://www.w3.org/2000/svg" width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="3" stroke-linecap="round" stroke-linejoin="round"><path d="M20 6 9 17l-5-5"/></svg>'

/** A prévia da nota na ordem do documento, com os limites do Ideario e "+N itens" para as tarefas que ficaram de fora. */
export function previewBlocks(note: string): (Block | { kind: 'more'; count: number })[] {
  const out: (Block | { kind: 'more'; count: number })[] = []
  let tasks = 0
  let chars = 0
  let hidden = 0
  let afterTask: number | null = null
  for (const b of parseNote(note)) {
    const full = out.length >= MAX_BLOCKS || chars >= MAX_CHARS
    if (b.kind === 'task') {
      if (tasks >= MAX_TASKS || full) {
        hidden++
        continue
      }
      tasks++
      out.push(b)
      afterTask = out.length
    } else if (!full) out.push(b)
    chars += b.runs.reduce((n, r) => n + r.text.length, 0)
  }
  if (hidden > 0) out.splice(afterTask ?? out.length, 0, { kind: 'more', count: hidden })
  return out
}

/** O miolo do card (o que vai dentro de `.card`), igual ao `NoteCard.svelte` do Ideario. */
export function cardHtml(title: string, note: string): string {
  const blocks = previewBlocks(note)
  const items = blocks.map((b) => {
    if (b.kind === 'more') return `<p class="pv-more">+${b.count} ${b.count === 1 ? 'item' : 'itens'}</p>`
    const depth = `style="--depth:${b.depth}"`
    switch (b.kind) {
      case 'task':
        return `<p class="pv-task${b.done ? ' done' : ''}" ${depth}><span class="box">${b.done ? CHECK : ''}</span><span>${runsHtml(b.runs)}</span></p>`
      case 'li':
      case 'ol':
        return `<p class="pv-li" ${depth} data-mark="${b.kind === 'ol' ? esc(b.mark ?? '1.') : '•'}">${runsHtml(b.runs)}</p>`
      case 'code':
        return `<pre class="pv-code">${runsHtml(b.runs)}</pre>`
      case 'h':
        return `<p class="pv-h">${runsHtml(b.runs)}</p>`
      default:
        return `<p class="pv-p">${runsHtml(b.runs)}</p>`
    }
  })
  return `<div class="card-body"><h3>${esc(title)}</h3>${items.length ? `<div class="preview">${items.join('')}</div>` : ''}</div>`
}

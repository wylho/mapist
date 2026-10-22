// Nó com nota = o card de nota do Ideario, que agora mora no Atrio (`@wylho/atrio`): na tela, o próprio `CardBody`
// dentro de um `.card`, por cima do canvas (`Canvas.svelte`); a prévia vem do núcleo (`view`, a regra do Atrio), com as mesmas
// regras da projeção do Ideario. Nó sem nota continua sendo a pílula de sempre.
//
// Aqui fica só o desenho do mesmo card em canvas, para a exportação (PNG, SVG, PDF saem só do desenho): `layoutCard`
// quebra as linhas em unidades de mundo com uma função de medir, e `render.ts` pinta.

import type { CardFields, CardView, FlatNode } from './api'
import type { CardSummary, NoteColor, PreviewBlock, RepeatKind } from '@wylho/atrio'

/** Medidas do `.card` do Atrio (raio 16, corpo 12/13, título 16/1.25, prévia 13.5/1.45, máx. 300 de prévia). */
export const CARD = {
  width: 240,
  radius: 16,
  padX: 13,
  padY: 12,
  gap: 6,
  titleSize: 16,
  titleLine: 1.25,
  bodySize: 13.5,
  bodyLine: 1.45,
  previewMax: 300,
}

/** O card tem pílulas: lembrete, status ou etiquetas. */
/** As etiquetas do card: as escolhidas e as `#palavras` da nota, sem repetir (como no Ideario). */
export function cardTags(c: CardFields | undefined, view?: CardView): string[] {
  return [...new Set([...(c?.tags ?? []), ...(view?.hash_tags ?? [])])]
}

export function hasPills(c: CardFields | undefined, view?: CardView): boolean {
  return (!!c && (c.reminder_at != null || !!c.status_on)) || cardTags(c, view).length > 0 || (view?.file_count ?? 0) > 0
}

/** Um tópico vira card quando tem nota, emoji ou pílulas (a raiz fica como está). */
export function isCard(n: { note: string; depth: number; card?: CardFields }): boolean {
  return n.depth > 0 && (n.note.trim().length > 0 || !!n.card?.emoji || hasPills(n.card))
}

export type LineKind = 'title' | 'heading' | 'text' | 'bullet' | 'ordered' | 'task' | 'code' | 'more' | 'file' | 'link'

/** Uma linha pronta para desenhar, relativa ao canto do card. */
export interface CardLine {
  kind: LineKind
  x: number
  /** Linha de base. */
  y: number
  text: string
  /** Primeira linha do bloco: leva o marcador (•, 1., caixa da tarefa). */
  first: boolean
  done?: boolean
  mark?: string
  markX: number
  /** Esmaecida no fim da prévia cortada, como a máscara do Ideario. */
  alpha: number
}

export interface CardLayout {
  width: number
  height: number
  lines: CardLine[]
}

/** Mede um trecho na fonte pedida. */
export type Measure = (text: string, size: number, weight: number, mono?: boolean) => number

/** Quebra um texto em linhas de até `max`, palavra por palavra (uma palavra maior que a linha é cortada). */
function wrap(text: string, max: number, size: number, weight: number, measure: Measure, mono = false): string[] {
  const out: string[] = []
  for (const para of text.split('\n')) {
    let line = ''
    for (const word of para.split(/\s+/).filter(Boolean)) {
      const next = line ? `${line} ${word}` : word
      if (line && measure(next, size, weight, mono) > max) {
        out.push(line)
        line = word
      } else line = next
      while (measure(line, size, weight, mono) > max && line.length > 1) {
        let cut = line
        while (cut.length > 1 && measure(cut, size, weight, mono) > max) cut = cut.slice(0, -1)
        out.push(cut)
        line = line.slice(cut.length)
      }
    }
    out.push(line)
  }
  return out
}

/** Posiciona título e prévia dentro do card. */
export function layoutCard(title: string, blocks: PreviewBlock[], measure: Measure): CardLayout {
  const inner = CARD.width - CARD.padX * 2
  const lines: CardLine[] = []
  let y = CARD.padY
  for (const t of wrap(title || ' ', inner, CARD.titleSize, 600, measure)) {
    lines.push({ kind: 'title', x: CARD.padX, y: y + CARD.titleSize, text: t, first: true, markX: CARD.padX, alpha: 1 })
    y += CARD.titleSize * CARD.titleLine
  }
  y += CARD.gap
  const top = y
  const step = CARD.bodySize * CARD.bodyLine
  let clipped = false
  for (const b of blocks) {
    const depth = 'depth' in b ? b.depth * 12 : 0
    const indent = b.kind === 'bullet' ? 14 : b.kind === 'ordered' ? 18 : b.kind === 'task' ? 22 : 0
    const text = b.kind === 'more' ? `+${b.count} ${b.count === 1 ? 'item' : 'itens'}` : b.text
    const size = b.kind === 'code' ? 11.5 : b.kind === 'more' ? 12 : CARD.bodySize
    const weight = b.kind === 'heading' ? 650 : 400
    if (b.kind === 'heading' && y > top) y += 6
    wrap(text, inner - depth - indent, size, weight, measure, b.kind === 'code').forEach((t, i) => {
      if (y + step - top > CARD.previewMax) {
        clipped = true
        return
      }
      lines.push({
        kind: b.kind,
        x: CARD.padX + depth + indent,
        y: y + size + (step - size) / 2 - 1,
        text: t,
        first: i === 0,
        done: b.kind === 'task' ? b.done : undefined,
        mark: b.kind === 'bullet' ? '•' : b.kind === 'ordered' ? `${b.n}.` : undefined,
        markX: CARD.padX + depth,
        alpha: 1,
      })
      y += step
    })
    y += 3
  }
  if (clipped) {
    const body = lines.filter((l) => l.kind !== 'title')
    if (body.length) body[body.length - 1].alpha = 0.2
    if (body.length > 1) body[body.length - 2].alpha = 0.55
  }
  return { width: CARD.width, height: Math.ceil(y - 3 + CARD.padY), lines }
}

/**
 * O resumo que o card padrão do Atrio (`NoteCard`) mostra, a partir do tópico: o título é o texto do nó, a prévia e a
 * capa vêm do núcleo (`view`), as etiquetas juntam as escolhidas e as `#palavras` da nota.
 */
export function summaryOf(n: Pick<FlatNode, 'text' | 'card' | 'view'>, color: NoteColor): CardSummary {
  const c = n.card ?? {}
  return {
    title: n.text,
    emoji: c.emoji ?? null,
    color,
    preview: n.view?.preview ?? [],
    cover: n.view?.cover ?? [],
    imageCount: n.view?.image_count ?? 0,
    fileCount: n.view?.file_count ?? 0,
    tags: cardTags(c, n.view),
    reminderAt: c.reminder_at ?? null,
    reminderRepeat: (c.reminder_repeat ?? null) as RepeatKind | null,
    reminderDone: !!c.reminder_done,
    reminderDoing: !!c.reminder_doing,
    statusOn: !!c.status_on,
  }
}


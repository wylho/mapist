// Tópico com nota (ou emoji, ou pílulas) = o card padrão do Atrio (`NoteCard`), o mesmo do Ideario: na tela, por cima
// do canvas (`Canvas.svelte`); na exportação, o mesmo card lido da tela (`domcard.ts`). A prévia vem do núcleo
// (`view`, a regra do Atrio). Tópico sem nada disso continua sendo a pílula de sempre.

import type { CardFields, CardView, FlatNode } from './api'
import type { CardSummary, NoteColor, RepeatKind } from '@wylho/atrio'

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


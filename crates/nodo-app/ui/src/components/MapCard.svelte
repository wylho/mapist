<script lang="ts">
  // O miolo do card de um tópico no mapa, igual ao card da lista do Ideario: emoji, título, prévia da nota e as pílulas
  // (lembrete ou status, etiquetas). O mesmo componente desenha na tela (`Canvas.svelte`) e mede (`measure.ts`).
  import { CardBody, CardPills, type PreviewBlock, type RepeatKind } from '@wylho/atrio'
  import type { CardFields } from '../lib/api'
  import { hasPills } from '../lib/cardtext'

  let { title, blocks, card = {} }: { title: string; blocks: PreviewBlock[]; card?: CardFields } = $props()
</script>

{#snippet pills()}<CardPills
    reminderAt={card.reminder_at ?? null}
    reminderRepeat={(card.reminder_repeat ?? null) as RepeatKind | null}
    reminderDone={!!card.reminder_done}
    reminderDoing={!!card.reminder_doing}
    statusOn={!!card.status_on}
    tags={card.tags ?? []}
  />{/snippet}

<CardBody {title} emoji={card.emoji ?? null} {blocks} meta={hasPills(card) ? pills : undefined} />

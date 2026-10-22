<script lang="ts">
  // O miolo do card de um tópico no mapa, igual ao card da lista do Ideario: emoji, título, prévia da nota e as pílulas
  // (lembrete ou status, etiquetas). O mesmo componente desenha na tela (`Canvas.svelte`) e mede (`measure.ts`; lá
  // com um `onstatus` vazio, para o seletor de status ocupar o mesmo lugar). A capa é a primeira foto ou fileira de
  // fotos da nota, como no Ideario.
  import { CardBody, CardPills, type PreviewBlock, type ReminderStatus, type RepeatKind } from '@wylho/atrio'
  import { attachments, type CardFields } from '../lib/api'
  import { hasPills } from '../lib/cardtext'

  let {
    title,
    blocks,
    cover = [],
    card = {},
    onstatus,
  }: { title: string; blocks: PreviewBlock[]; cover?: string[]; card?: CardFields; onstatus?: (to: ReminderStatus) => void } = $props()
</script>

{#snippet pills()}<CardPills
    reminderAt={card.reminder_at ?? null}
    reminderRepeat={(card.reminder_repeat ?? null) as RepeatKind | null}
    reminderDone={!!card.reminder_done}
    reminderDoing={!!card.reminder_doing}
    statusOn={!!card.status_on}
    tags={card.tags ?? []}
    {onstatus}
  />{/snippet}

{#if cover.length}
  <!-- Proporção fixa no mapa: o card é medido antes de a foto carregar, e a altura não pode depender dela. -->
  <div class="card-media map-cover" class:row={cover.length > 1}>
    {#each cover.slice(0, 4) as h (h)}<img src={attachments.url(h)} alt="" decoding="async" />{/each}
  </div>
{/if}
<CardBody {title} emoji={card.emoji ?? null} {blocks} meta={hasPills(card) ? pills : undefined} />

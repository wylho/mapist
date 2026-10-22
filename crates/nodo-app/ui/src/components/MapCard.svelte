<script lang="ts">
  // O miolo do card de um tópico no mapa, igual ao card da lista do Ideario: capa, emoji, título, prévia da nota e as
  // pílulas (lembrete ou status, anexos, etiquetas). A prévia e a capa vêm prontas do núcleo (`view`), pela regra do
  // Atrio, a mesma do Ideario. O mesmo componente desenha na tela (`Canvas.svelte`) e mede (`measure.ts`; lá com um
  // `onstatus` vazio, para o seletor de status ocupar o mesmo lugar). A capa leva a proporção de cada foto, então a
  // altura do card é a mesma antes e depois de a foto carregar.
  import { CardBody, CardPills, type ReminderStatus, type RepeatKind } from '@wylho/atrio'
  import { attachments, type CardFields, type CardView } from '../lib/api'
  import { cardTags, hasPills } from '../lib/cardtext'

  let {
    title,
    view,
    card = {},
    onstatus,
  }: { title: string; view?: CardView; card?: CardFields; onstatus?: (to: ReminderStatus) => void } = $props()

  const tags = $derived(cardTags(card, view))
  const files = $derived(view?.file_count ?? 0)
  const cover = $derived(view?.cover ?? [])
  const ratio = (c: { width: number | null; height: number | null }) => (c.width && c.height ? c.width / c.height : 16 / 10)
</script>

{#snippet pills()}<CardPills
    reminderAt={card.reminder_at ?? null}
    reminderRepeat={(card.reminder_repeat ?? null) as RepeatKind | null}
    reminderDone={!!card.reminder_done}
    reminderDoing={!!card.reminder_doing}
    statusOn={!!card.status_on}
    fileCount={files}
    {tags}
    {onstatus}
  />{/snippet}

{#if cover.length}
  <!-- Uma foto, ou a primeira linha de fotos lado a lado: mesma altura, larguras pela proporção de cada uma. -->
  <div class="card-media" class:row={cover.length > 1}>
    {#each cover as c (c.hash)}
      <img src={attachments.thumb(c.hash)} alt="" style:aspect-ratio={ratio(c)} style:flex-grow={ratio(c)} decoding="async" />
    {/each}
    {#if (view?.image_count ?? 0) > cover.length}<span class="more">+{(view?.image_count ?? 0) - cover.length}</span>{/if}
  </div>
{/if}
<CardBody {title} emoji={card.emoji ?? null} blocks={view?.preview ?? []} meta={hasPills(card, view) ? pills : undefined} />

<script lang="ts">
  // Exportar o mapa: escolhe o formato, ajusta as opções que valem para ele e grava. PNG, SVG e PDF saem do mesmo
  // desenho da tela; Markdown e OPML vêm do núcleo, iguais aos que o Claude exporta.
  import { Dialog, ToggleGroup } from 'bits-ui'
  import { Download, FileCode2, FileImage, FileText, FileType2, ListTree, X } from '@lucide/svelte'
  import { t } from '@wylho/atrio'
  import { doc } from '../lib/document.svelte'
  import { ui } from '../lib/ui.svelte'
  import { FORMATS, exportTo, pickOrientation, type CardsMode, type ExportFormat, type Orientation, type Paper } from '../lib/exporter'

  const ICONS = { png: FileImage, svg: FileCode2, pdf: FileType2, markdown: FileText, opml: ListTree }

  let format = $state<ExportFormat>('png')
  let scale = $state('2')
  let background = $state(true)
  let orientation = $state<Orientation>('auto')
  let paper = $state<Paper>('a4')
  let cards = $state<CardsMode>('full')
  let unfold = $state(false)
  const hasFolded = $derived(doc.snapshot.nodes.some((n) => n.folded))
  let busy = $state(false)

  const autoIs = $derived(pickOrientation(doc.snapshot, 'auto') === 'landscape' ? t('paisagem') : t('retrato'))

  async function run() {
    if (busy || !doc.hasDocument) return
    busy = true
    try {
      const where = await exportTo(format, doc.snapshot, { scale: Number(scale), background, orientation, paper, cards, unfold })
      if (where) {
        ui.exportOpen = false
        ui.say(t('Exportado: {nome}', { nome: where.split(/[\\/]/).pop() ?? where }))
      }
    } catch (e) {
      ui.say(t('Não foi possível exportar: {erro}', { erro: String(e instanceof Error ? e.message : e) }))
    } finally {
      busy = false
    }
  }
</script>

<Dialog.Root bind:open={ui.exportOpen}>
  <Dialog.Portal>
    <Dialog.Overlay class="overlay" />
    <Dialog.Content class="sheet" aria-describedby={undefined}>
      <div class="sheet-head">
        <Dialog.Title class="sheet-title">{t('Exportar')}</Dialog.Title>
        <Dialog.Close class="icon-btn" aria-label={t('Fechar')}><X size={20} /></Dialog.Close>
      </div>
      <div class="sheet-scroll">
        <div class="export-list" role="radiogroup" aria-label={t('Formato')}>
          {#each FORMATS as f (f.id)}
            {@const Icon = ICONS[f.id]}
            <button class="export-item" role="radio" aria-checked={format === f.id} onclick={() => (format = f.id)}>
              <span class="export-ico"><Icon size={18} /></span>
              <span class="grow"><b>{t(f.label)}</b><small>{t(f.hint)}</small></span>
              <span class="export-ext">.{f.ext}</span>
            </button>
          {/each}
        </div>

        {#if format === 'png' || format === 'svg' || format === 'pdf'}
          <div class="set-row">
            <span class="grow"><b>{t('Cards')}</b><small>{t('Inteiros como na tela, ou só o título de cada tópico (um mapa mais limpo).')}</small></span>
            <ToggleGroup.Root type="single" value={cards} onValueChange={(v) => v && (cards = v as CardsMode)} class="seg tight" aria-label={t('Cards')}>
              <ToggleGroup.Item value="full" class="seg-item">{t('Inteiros')}</ToggleGroup.Item>
              <ToggleGroup.Item value="title" class="seg-item">{t('Só o título')}</ToggleGroup.Item>
            </ToggleGroup.Root>
          </div>
          {#if hasFolded}
            <div class="set-row">
              <span class="grow"><b>{t('Ramos recolhidos')}</b><small>{t('Como estão no mapa, ou todos abertos na imagem (o mapa não muda).')}</small></span>
              <ToggleGroup.Root type="single" value={unfold ? 'all' : 'keep'} onValueChange={(v) => v && (unfold = v === 'all')} class="seg tight" aria-label={t('Ramos recolhidos')}>
                <ToggleGroup.Item value="keep" class="seg-item">{t('Como estão')}</ToggleGroup.Item>
                <ToggleGroup.Item value="all" class="seg-item">{t('Expandir tudo')}</ToggleGroup.Item>
              </ToggleGroup.Root>
            </div>
          {/if}
        {/if}
        {#if format === 'png'}
          <div class="set-row">
            <span class="grow"><b>{t('Tamanho')}</b><small>{t('2× fica nítido em telas de alta resolução.')}</small></span>
            <ToggleGroup.Root type="single" value={scale} onValueChange={(v) => v && (scale = v)} class="seg tight" aria-label={t('Tamanho')}>
              <ToggleGroup.Item value="1" class="seg-item">1×</ToggleGroup.Item>
              <ToggleGroup.Item value="2" class="seg-item">2×</ToggleGroup.Item>
              <ToggleGroup.Item value="3" class="seg-item">3×</ToggleGroup.Item>
            </ToggleGroup.Root>
          </div>
        {/if}
        {#if format === 'png' || format === 'svg'}
          <div class="set-row">
            <span class="grow"><b>{t('Fundo')}</b><small>{t('Transparente para colocar sobre outra cor.')}</small></span>
            <ToggleGroup.Root type="single" value={background ? 'on' : 'off'} onValueChange={(v) => v && (background = v === 'on')} class="seg tight" aria-label={t('Fundo')}>
              <ToggleGroup.Item value="on" class="seg-item">{t('Com fundo')}</ToggleGroup.Item>
              <ToggleGroup.Item value="off" class="seg-item">{t('Transparente')}</ToggleGroup.Item>
            </ToggleGroup.Root>
          </div>
        {/if}
        {#if format === 'pdf'}
          <div class="set-row">
            <span class="grow"><b>{t('Orientação')}</b><small>{t('Automática escolhe pelo formato do mapa (agora: {atual}). O mapa sempre cabe na página.', { atual: autoIs })}</small></span>
            <ToggleGroup.Root type="single" value={orientation} onValueChange={(v) => v && (orientation = v as Orientation)} class="seg tight" aria-label={t('Orientação')}>
              <ToggleGroup.Item value="auto" class="seg-item">{t('Automática')}</ToggleGroup.Item>
              <ToggleGroup.Item value="portrait" class="seg-item">{t('Retrato')}</ToggleGroup.Item>
              <ToggleGroup.Item value="landscape" class="seg-item">{t('Paisagem')}</ToggleGroup.Item>
            </ToggleGroup.Root>
          </div>
          <div class="set-row">
            <span class="grow"><b>{t('Papel')}</b></span>
            <ToggleGroup.Root type="single" value={paper} onValueChange={(v) => v && (paper = v as Paper)} class="seg tight" aria-label={t('Papel')}>
              <ToggleGroup.Item value="a4" class="seg-item">A4</ToggleGroup.Item>
              <ToggleGroup.Item value="letter" class="seg-item">{t('Carta')}</ToggleGroup.Item>
            </ToggleGroup.Root>
          </div>
        {/if}

        <div class="export-go">
          <button class="btn primary" disabled={busy || !doc.hasDocument} onclick={run}><Download size={17} />{busy ? t('Exportando…') : t('Exportar')}</button>
        </div>
      </div>
    </Dialog.Content>
  </Dialog.Portal>
</Dialog.Root>

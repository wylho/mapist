<script lang="ts">
  // Exportar o mapa: escolhe o formato, ajusta as opções que valem para ele e grava. PNG, SVG e PDF saem do mesmo
  // desenho da tela; Markdown e OPML vêm do núcleo, iguais aos que o Claude exporta.
  import { Dialog, ToggleGroup } from 'bits-ui'
  import { Download, FileCode2, FileImage, FileText, FileType2, ListTree, X } from '@lucide/svelte'
  import { doc } from '../lib/document.svelte'
  import { ui } from '../lib/ui.svelte'
  import { FORMATS, exportTo, pickOrientation, type ExportFormat, type Orientation, type Paper } from '../lib/exporter'

  const ICONS = { png: FileImage, svg: FileCode2, pdf: FileType2, markdown: FileText, opml: ListTree }

  let format = $state<ExportFormat>('png')
  let scale = $state('2')
  let background = $state(true)
  let orientation = $state<Orientation>('auto')
  let paper = $state<Paper>('a4')
  let busy = $state(false)

  const autoIs = $derived(pickOrientation(doc.snapshot, 'auto') === 'landscape' ? 'paisagem' : 'retrato')

  async function run() {
    if (busy || !doc.hasDocument) return
    busy = true
    try {
      const where = await exportTo(format, doc.snapshot, { scale: Number(scale), background, orientation, paper })
      if (where) {
        ui.exportOpen = false
        ui.say(`Exportado: ${where.split(/[\\/]/).pop()}`)
      }
    } catch (e) {
      ui.say(`Não foi possível exportar: ${e instanceof Error ? e.message : e}`)
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
        <Dialog.Title class="sheet-title">Exportar</Dialog.Title>
        <Dialog.Close class="icon-btn" aria-label="Fechar"><X size={20} /></Dialog.Close>
      </div>
      <div class="sheet-scroll">
        <div class="export-list" role="radiogroup" aria-label="Formato">
          {#each FORMATS as f (f.id)}
            {@const Icon = ICONS[f.id]}
            <button class="export-item" role="radio" aria-checked={format === f.id} onclick={() => (format = f.id)}>
              <span class="export-ico"><Icon size={18} /></span>
              <span class="grow"><b>{f.label}</b><small>{f.hint}</small></span>
              <span class="export-ext">.{f.ext}</span>
            </button>
          {/each}
        </div>

        {#if format === 'png'}
          <div class="set-row">
            <span class="grow"><b>Tamanho</b><small>2× fica nítido em telas de alta resolução.</small></span>
            <ToggleGroup.Root type="single" value={scale} onValueChange={(v) => v && (scale = v)} class="seg tight" aria-label="Tamanho">
              <ToggleGroup.Item value="1" class="seg-item">1×</ToggleGroup.Item>
              <ToggleGroup.Item value="2" class="seg-item">2×</ToggleGroup.Item>
              <ToggleGroup.Item value="3" class="seg-item">3×</ToggleGroup.Item>
            </ToggleGroup.Root>
          </div>
        {/if}
        {#if format === 'png' || format === 'svg'}
          <div class="set-row">
            <span class="grow"><b>Fundo</b><small>Transparente para colocar sobre outra cor.</small></span>
            <ToggleGroup.Root type="single" value={background ? 'on' : 'off'} onValueChange={(v) => v && (background = v === 'on')} class="seg tight" aria-label="Fundo">
              <ToggleGroup.Item value="on" class="seg-item">Com fundo</ToggleGroup.Item>
              <ToggleGroup.Item value="off" class="seg-item">Transparente</ToggleGroup.Item>
            </ToggleGroup.Root>
          </div>
        {/if}
        {#if format === 'pdf'}
          <div class="set-row">
            <span class="grow"><b>Orientação</b><small>Automática escolhe pelo formato do mapa (agora: {autoIs}). O mapa sempre cabe na página.</small></span>
            <ToggleGroup.Root type="single" value={orientation} onValueChange={(v) => v && (orientation = v as Orientation)} class="seg tight" aria-label="Orientação">
              <ToggleGroup.Item value="auto" class="seg-item">Automática</ToggleGroup.Item>
              <ToggleGroup.Item value="portrait" class="seg-item">Retrato</ToggleGroup.Item>
              <ToggleGroup.Item value="landscape" class="seg-item">Paisagem</ToggleGroup.Item>
            </ToggleGroup.Root>
          </div>
          <div class="set-row">
            <span class="grow"><b>Papel</b></span>
            <ToggleGroup.Root type="single" value={paper} onValueChange={(v) => v && (paper = v as Paper)} class="seg tight" aria-label="Papel">
              <ToggleGroup.Item value="a4" class="seg-item">A4</ToggleGroup.Item>
              <ToggleGroup.Item value="letter" class="seg-item">Carta</ToggleGroup.Item>
            </ToggleGroup.Root>
          </div>
        {/if}

        <div class="export-go">
          <button class="btn primary" disabled={busy || !doc.hasDocument} onclick={run}><Download size={17} />{busy ? 'Exportando…' : 'Exportar'}</button>
        </div>
      </div>
    </Dialog.Content>
  </Dialog.Portal>
</Dialog.Root>

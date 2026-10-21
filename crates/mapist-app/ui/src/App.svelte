<script lang="ts">
  // Raiz da aplicação: barra, canvas, inspetor e atalhos globais.
  import { onMount } from 'svelte'
  import { X, Minus, Plus, Maximize } from '@lucide/svelte'
  import Toolbar from './components/Toolbar.svelte'
  import Canvas from './components/Canvas.svelte'
  import Inspector from './components/Inspector.svelte'
  import ThemeSheet from './components/ThemeSheet.svelte'
  import { doc } from './lib/document.svelte'
  import { hasHost, isPreview } from './lib/api'
  import { prefs } from './lib/prefs.svelte'

  let canvasRef = $state<Canvas | undefined>(undefined)
  // O nível de zoom vive no canvas; espelhamos aqui para o rótulo reagir.
  let zoom = $state(prefs.zoom)

  const zoomPct = $derived(Math.round(zoom * 100))

  function onKeydown(e: KeyboardEvent) {
    const target = e.target as HTMLElement | null
    const typing =
      target &&
      (target.tagName === 'TEXTAREA' || target.tagName === 'INPUT' || target.isContentEditable)
    const mod = e.metaKey || e.ctrlKey

    if (mod && e.key.toLowerCase() === 'z') {
      e.preventDefault()
      e.shiftKey ? doc.redo() : doc.undo()
      return
    }
    if (mod && e.key.toLowerCase() === 's') {
      e.preventDefault()
      doc.saveDialog(e.shiftKey)
      return
    }
    if (mod && e.key.toLowerCase() === 'o') {
      e.preventDefault()
      doc.openDialog()
      return
    }
    if (mod && e.key.toLowerCase() === 'n') {
      e.preventDefault()
      doc.newMap()
      return
    }
    if (typing) return

    switch (e.key) {
      case 'Tab':
        e.preventDefault()
        doc.addChild()
        break
      case 'Enter':
        e.preventDefault()
        doc.addSibling()
        break
      case 'Backspace':
      case 'Delete':
        e.preventDefault()
        doc.remove()
        break
      case ' ':
        e.preventDefault()
        doc.toggleFold()
        break
      case 'ArrowUp':
        e.preventDefault()
        doc.moveSelection(0, -1)
        break
      case 'ArrowDown':
        e.preventDefault()
        doc.moveSelection(0, 1)
        break
      case 'ArrowLeft':
        e.preventDefault()
        doc.moveSelection(-1, 0)
        break
      case 'ArrowRight':
        e.preventDefault()
        doc.moveSelection(1, 0)
        break
    }
  }

  onMount(() => {
    doc.load()
  })
</script>

<svelte:window onkeydown={onKeydown} />

<div class="map-shell">
  <Toolbar />

  <div class="map-body">
    <Canvas bind:this={canvasRef} onzoom={(z) => { zoom = z; prefs.setZoom(z) }}>
      <ThemeSheet />

      <div class="scope-bar">
        <span class="scope-pill">
          <span class="dot"></span>
          <span class="grow">{doc.snapshot.title || 'Sem título'}{doc.snapshot.dirty ? ' •' : ''}</span>
          <button aria-label="Limpar seleção" title="Limpar seleção" onclick={() => (doc.selected = null)}>
            <X size={12} strokeWidth={2.5} />
          </button>
        </span>
      </div>

      <div class="zoombar">
        <button aria-label="Diminuir zoom" title="Diminuir" onclick={() => canvasRef?.zoomBy(1 / 1.2)}>
          <Minus size={17} />
        </button>
        <span class="zoomlevel">{zoomPct}%</span>
        <button aria-label="Aumentar zoom" title="Aumentar" onclick={() => canvasRef?.zoomBy(1.2)}>
          <Plus size={17} />
        </button>
        <button aria-label="Ajustar à tela" title="Ajustar à tela" onclick={() => canvasRef?.fit(true)}>
          <Maximize size={17} />
        </button>
      </div>
    </Canvas>

    {#if prefs.inspectorOpen}
      <Inspector />
    {/if}
  </div>

  {#if doc.error}
    <div class="banner" role="alert" style="position:absolute;bottom:16px;left:16px;max-width:420px;z-index:9">
      {doc.error}
    </div>
  {/if}

  {#if isPreview}
    <div class="preview-tag" title="Backend simulado em memória; nenhum arquivo é lido ou gravado">
      <span class="dot"></span> Preview — dados de exemplo
    </div>
  {:else if !hasHost}
    <div class="banner" style="position:absolute;top:76px;left:50%;transform:translateX(-50%);z-index:9">
      Rodando fora do Tauri — sem acesso ao documento
    </div>
  {/if}
</div>

<script lang="ts">
  // Raiz da aplicação: barra, canvas, inspetor e atalhos globais.
  import { onMount } from 'svelte'
  import Toolbar from './components/Toolbar.svelte'
  import Canvas from './components/Canvas.svelte'
  import Inspector from './components/Inspector.svelte'
  import ThemeSheet from './components/ThemeSheet.svelte'
  import { doc } from './lib/document.svelte'
  import { hasHost } from './lib/api'
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
      doc.save(doc.snapshot.path ?? undefined)
      return
    }
    if (mod && e.key.toLowerCase() === 'n') {
      e.preventDefault()
      doc.create('Tópico central')
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
          <button title="Limpar seleção" onclick={() => (doc.selected = null)}>
            <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round"><path d="M18 6 6 18M6 6l12 12" /></svg>
          </button>
        </span>
      </div>

      <div class="zoombar">
        <button title="Diminuir" onclick={() => canvasRef?.zoomBy(1 / 1.2)}>
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><path d="M5 12h14" /></svg>
        </button>
        <span class="zoomlevel">{zoomPct}%</span>
        <button title="Aumentar" onclick={() => canvasRef?.zoomBy(1.2)}>
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><path d="M12 5v14M5 12h14" /></svg>
        </button>
        <button title="Ajustar à tela" onclick={() => canvasRef?.fit(true)}>
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M8 3H5a2 2 0 0 0-2 2v3M16 3h3a2 2 0 0 1 2 2v3M16 21h3a2 2 0 0 0 2-2v-3M8 21H5a2 2 0 0 1-2-2v-3" /></svg>
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

  {#if !hasHost}
    <div class="banner" style="position:absolute;top:76px;left:50%;transform:translateX(-50%);z-index:9">
      Rodando fora do Tauri — sem acesso ao documento
    </div>
  {/if}
</div>

<script lang="ts">
  // Canvas do mapa: desenha, faz pan/zoom, seleciona e arrasta nós.
  import { onMount } from 'svelte'
  import { doc } from '../lib/document.svelte'
  import { theme } from '../lib/theme.svelte'
  import { draw, hitTest, readThemeColors, type Viewport } from '../lib/render'
  import { prefs } from '../lib/prefs.svelte'
  import { levelStyle, mapTheme } from '../lib/maptheme'

  import type { Snippet } from 'svelte'

  let { children, onzoom }: { children?: Snippet; onzoom?: (z: number) => void } = $props()

  let canvas: HTMLCanvasElement
  let stageEl: HTMLDivElement
  let editorEl = $state<HTMLInputElement | undefined>(undefined)


  const view = $state<Viewport>({ x: 0, y: 0, scale: 1 })
  let colors = $state(readThemeColors())
  let panning: { x: number; y: number } | null = null
  let dragging: { id: number; dx: number; dy: number; ox: number; oy: number } | null = null
  let draggedFar = false

  export function zoomBy(factor: number, cx?: number, cy?: number) {
    const r = stageEl.getBoundingClientRect()
    const px = cx ?? r.width / 2
    const py = cy ?? r.height / 2
    const wx = (px - view.x) / view.scale
    const wy = (py - view.y) / view.scale
    view.scale = Math.min(4, Math.max(0.1, view.scale * factor))
    view.x = px - wx * view.scale
    view.y = py - wy * view.scale
    render()
  }

  export function fit(animate = true) {
    const b = doc.snapshot.bounds
    const r = stageEl.getBoundingClientRect()
    if (!doc.hasDocument) return
    // Margem proporcional: 110 px fixos numa tela de celular deixavam o mapa em 16%.
    const pad = Math.min(110, Math.max(16, Math.min(r.width, r.height) * 0.08))
    const w = b.max_x - b.min_x || 1
    const h = b.max_y - b.min_y || 1
    const target = Math.max(0.12, Math.min((r.width - pad * 2) / w, (r.height - pad * 2) / h, 1.35))
    const to = {
      scale: target,
      x: r.width / 2 - ((b.min_x + b.max_x) / 2) * target,
      y: r.height / 2 - ((b.min_y + b.max_y) / 2) * target,
    }
    if (!animate) {
      Object.assign(view, to)
      render()
      return
    }
    const from = { ...view }
    const t0 = performance.now()
    const dur = 320
    const step = (t: number) => {
      const k = Math.min(1, (t - t0) / dur)
      const e = 1 - Math.pow(1 - k, 3)
      view.scale = from.scale + (to.scale - from.scale) * e
      view.x = from.x + (to.x - from.x) * e
      view.y = from.y + (to.y - from.y) * e
      render()
      if (k < 1) requestAnimationFrame(step)
    }
    requestAnimationFrame(step)
  }

  function render() {
    const r = stageEl?.getBoundingClientRect()
    if (!canvas || !r) return
    const dpr = window.devicePixelRatio || 1
    const needW = Math.floor(r.width * dpr)
    const needH = Math.floor(r.height * dpr)
    if (canvas.width !== needW || canvas.height !== needH) {
      canvas.width = needW
      canvas.height = needH
      canvas.style.width = r.width + 'px'
      canvas.style.height = r.height + 'px'
    }
    const ctx = canvas.getContext('2d')
    if (!ctx) return
    draw({
      snapshot: doc.snapshot,
      linkStyle: prefs.linkStyle,
      mode: theme.mode,
      colors,
      view,
      selected: doc.selected,
      hovered: doc.hovered,
      matches: doc.matches,
      width: r.width,
      height: r.height,
      dpr,
      ...({ ctx } as Record<string, unknown>),
    } as never)
  }

  // Redesenha quando o documento ou o tema mudam.
  $effect(() => {
    void doc.snapshot
    void doc.selected
    void doc.hovered
    void doc.matches
    void theme.palette
    void theme.pref
    void theme.simDesktop
    void prefs.linkStyle
    colors = readThemeColors()
    render()
  })

  function onPointerDown(e: PointerEvent) {
    const r = stageEl.getBoundingClientRect()
    const hit = hitTest(doc.snapshot, view, e.clientX, e.clientY, r)
    if (hit !== null) {
      doc.selected = hit
      const n = doc.node(hit)
      if (n) {
        dragging = {
          id: hit,
          dx: e.clientX - (n.x * view.scale + view.x),
          dy: e.clientY - (n.y * view.scale + view.y),
          ox: n.x,
          oy: n.y,
        }
        draggedFar = false
        stageEl.classList.add('moving')
      }
      return
    }
    panning = { x: e.clientX - view.x, y: e.clientY - view.y }
    stageEl.classList.add('panning')
  }

  function onPointerMove(e: PointerEvent) {
    if (dragging) {
      const n = doc.node(dragging.id)
      if (!n) return
      n.x = (e.clientX - dragging.dx - view.x) / view.scale
      n.y = (e.clientY - dragging.dy - view.y) / view.scale
      // Só vale como edição depois de um movimento perceptível, senão todo
      // clique viraria uma entrada no histórico de desfazer.
      if (Math.abs(n.x - dragging.ox) > 2 || Math.abs(n.y - dragging.oy) > 2) draggedFar = true
      render()
      return
    }
    if (panning) {
      view.x = e.clientX - panning.x
      view.y = e.clientY - panning.y
      render()
      return
    }
    const r = stageEl.getBoundingClientRect()
    const hit = hitTest(doc.snapshot, view, e.clientX, e.clientY, r)
    if (hit !== doc.hovered) doc.hovered = hit
  }

  function onPointerUp() {
    // Um arrasto de verdade vira uma edição desfazível; um clique não.
    if (dragging && draggedFar) {
      const n = doc.node(dragging.id)
      if (n) doc.move(n.id, n.x, n.y)
    }
    panning = null
    dragging = null
    draggedFar = false
    stageEl?.classList.remove('panning', 'moving')
  }

  function onWheel(e: WheelEvent) {
    e.preventDefault()
    const r = stageEl.getBoundingClientRect()
    zoomBy(e.deltaY < 0 ? 1.12 : 1 / 1.12, e.clientX - r.left, e.clientY - r.top)
  }

  function onDblClick(e: MouseEvent) {
    const r = stageEl.getBoundingClientRect()
    const hit = hitTest(doc.snapshot, view, e.clientX, e.clientY, r)
    if (hit === null) return
    doc.selected = hit
    doc.startEdit()
  }

  // Abre o campo no lugar do nó sempre que o documento pede (tecla, Tab/Enter, duplo clique).
  $effect(() => {
    const ed = doc.editing
    if (!ed) return
    requestAnimationFrame(() => {
      if (!editorEl) return
      editorEl.focus()
      if (ed.selectAll) editorEl.select()
      else editorEl.setSelectionRange(editorEl.value.length, editorEl.value.length)
    })
  })

  function onEditorKey(e: KeyboardEvent) {
    // Nada do que se digita aqui chega aos atalhos do mapa.
    e.stopPropagation()
    if (e.isComposing) return
    if (e.key === 'Enter') {
      e.preventDefault()
      void doc.commitEdit()
    } else if (e.key === 'Tab') {
      // Como no XMind: confirma e já cria o próximo (Tab filho, Shift+Tab irmão).
      e.preventDefault()
      void doc.commitEdit().then(() => (e.shiftKey ? doc.addSibling() : doc.addChild()))
    } else if (e.key === 'Escape') {
      e.preventDefault()
      doc.cancelEdit()
    }
  }

  /** Onde o campo fica, em coordenadas de tela, com o texto no mesmo tamanho que o nó tem na tela. */
  const editorBox = $derived.by(() => {
    const ed = doc.editing
    const n = ed ? doc.node(ed.id) : null
    if (!n) return null
    const style = levelStyle(mapTheme(doc.snapshot.style), n.depth)
    return {
      left: n.x * view.scale + view.x,
      top: n.y * view.scale + view.y,
      width: Math.max(60, n.width * view.scale),
      height: Math.max(24, n.height * view.scale),
      font: style.size * view.scale,
      weight: style.weight,
    }
  })

  let ro: ResizeObserver | undefined

  onMount(() => {
    colors = readThemeColors()
    // Retoma a escala da sessão anterior; `fit` a substitui se for a estreia.
    view.scale = prefs.zoom
    render()
    ro = new ResizeObserver(() => render())
    ro.observe(stageEl)
    return () => ro?.disconnect()
  })

  $effect(() => {
    onzoom?.(view.scale)
  })

  let framed = false

  $effect(() => {
    // Enquadra uma vez, quando o primeiro documento com nós chega. Não pode
    // depender do zoom ser exatamente 1: as preferências restauram o zoom da
    // sessão anterior, e aí o mapa nunca seria enquadrado.
    if (framed || !doc.hasDocument) return
    framed = true
    // Dois quadros: o primeiro deixa o ResizeObserver medir o palco, o segundo
    // enquadra sobre um tamanho já correto. Com um só, o fit usava o rect
    // anterior e o mapa ficava fora de escala.
    requestAnimationFrame(() => requestAnimationFrame(() => fit(false)))
  })
</script>

<div
  class="stage"
  bind:this={stageEl}
  onpointerdown={onPointerDown}
  onpointermove={onPointerMove}
  onpointerup={onPointerUp}
  onpointerleave={onPointerUp}
  onwheel={onWheel}
  ondblclick={onDblClick}
  role="application"
  aria-label="Mapa mental"
>
  <canvas bind:this={canvas}></canvas>
  {#if doc.editing && editorBox}
    <input
      class="inline-editor"
      bind:this={editorEl}
      bind:value={doc.editing.draft}
      style:left="{editorBox.left}px"
      style:top="{editorBox.top}px"
      style:width="{editorBox.width}px"
      style:height="{editorBox.height}px"
      style:font-size="{editorBox.font}px"
      style:font-weight={editorBox.weight}
      onkeydown={onEditorKey}
      onblur={() => void doc.commitEdit()}
      onpointerdown={(e) => e.stopPropagation()}
      aria-label="Texto do nó"
    />
  {/if}

  {@render children?.()}
</div>

<style>
  .stage {
    flex: 1;
    min-width: 0;
    position: relative;
    overflow: hidden;
    background: var(--stage);
    touch-action: none;
  }
  canvas {
    display: block;
    width: 100%;
    height: 100%;
  }
  /* Editor no lugar do nó: mesma caixa, sem moldura pesada. */
  .inline-editor {
    position: absolute;
    z-index: 5;
    margin: 0;
    padding: 0 8px;
    font-family: var(--font-body);
    text-align: center;
    color: var(--fg);
    background: var(--surface);
    border: 2px solid var(--accent);
    border-radius: 14px;
    outline: none;
    box-shadow: 0 0 0 4px var(--accent-soft);
  }
</style>

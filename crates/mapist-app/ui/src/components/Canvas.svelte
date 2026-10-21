<script lang="ts">
  // Canvas do mapa: desenha, faz pan/zoom, seleciona e arrasta nós.
  import { onMount } from 'svelte'
  import { doc } from '../lib/document.svelte'
  import { theme } from '../lib/theme.svelte'
  import { draw, hitTest, readThemeColors, type Viewport } from '../lib/render'

  import type { Snippet } from 'svelte'

  let { children, onzoom }: { children?: Snippet; onzoom?: (z: number) => void } = $props()

  let canvas: HTMLCanvasElement
  let stageEl: HTMLDivElement
  let editorEl = $state<HTMLInputElement | undefined>(undefined)

  /** Nó sendo editado no lugar, se houver. */
  let editing = $state<{ id: number; x: number; y: number; w: number; h: number } | null>(null)
  let draft = $state('')

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
    const pad = 110
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
    const n = doc.node(hit)
    if (!n) return
    draft = n.text
    editing = { id: hit, x: n.x, y: n.y, w: n.width, h: n.height }
    // O input é posicionado em coordenadas de tela, então acompanha o zoom.
    requestAnimationFrame(() => {
      editorEl?.focus()
      editorEl?.select()
    })
  }

  function commitEdit() {
    const ed = editing
    editing = null
    if (!ed) return
    const n = doc.node(ed.id)
    const next = draft.trim()
    if (!n || next === '' || next === n.text) return
    doc.setText(ed.id, next)
  }

  function cancelEdit() {
    editing = null
  }

  function onEditorKey(e: KeyboardEvent) {
    if (e.key === 'Enter') {
      e.preventDefault()
      commitEdit()
    } else if (e.key === 'Escape') {
      e.preventDefault()
      cancelEdit()
    }
    // Impede que Tab/Delete cheguem aos atalhos globais enquanto se digita.
    e.stopPropagation()
  }

  /** Onde o editor fica, em coordenadas de tela. */
  const editorBox = $derived.by(() => {
    if (!editing) return null
    return {
      left: editing.x * view.scale + view.x,
      top: editing.y * view.scale + view.y,
      width: Math.max(60, editing.w * view.scale),
      height: Math.max(28, editing.h * view.scale),
    }
  })

  let ro: ResizeObserver | undefined

  onMount(() => {
    colors = readThemeColors()
    render()
    ro = new ResizeObserver(() => render())
    ro.observe(stageEl)
    return () => ro?.disconnect()
  })

  $effect(() => {
    onzoom?.(view.scale)
  })

  $effect(() => {
    // Enquadra na primeira vez que um documento com nós aparece.
    if (doc.hasDocument && view.scale === 1 && view.x === 0 && view.y === 0) {
      requestAnimationFrame(() => fit(false))
    }
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
  {#if editing && editorBox}
    <input
      class="inline-editor"
      bind:this={editorEl}
      bind:value={draft}
      style:left="{editorBox.left}px"
      style:top="{editorBox.top}px"
      style:width="{editorBox.width}px"
      style:height="{editorBox.height}px"
      onkeydown={onEditorKey}
      onblur={commitEdit}
      aria-label="Editar texto do nó"
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
    font-size: 13.5px;
    font-weight: 600;
    text-align: center;
    color: var(--fg);
    background: var(--surface);
    border: 2px solid var(--accent);
    border-radius: 14px;
    outline: none;
    box-shadow: 0 0 0 4px var(--accent-soft);
  }
</style>

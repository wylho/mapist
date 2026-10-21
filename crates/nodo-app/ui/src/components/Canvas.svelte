<script lang="ts">
  // Canvas do mapa: desenha, faz pan/zoom, seleciona e arrasta nós.
  import { onMount } from 'svelte'
  import { doc } from '../lib/document.svelte'
  import { theme } from '../lib/theme.svelte'
  import { cardFill, draw, hitNoteDot, hitTest, readThemeColors, resolveColor, type Viewport } from '../lib/render'
  import { CARD, isCard } from '../lib/cardtext'
  import { previewFromMarkdown } from '@wylho/atrio'
  import MapCard from './MapCard.svelte'
  import { notes } from '../lib/notes.svelte'
  import NoteCard from './NoteCard.svelte'
  import NodeNote from './NodeNote.svelte'
  import { prefs } from '../lib/prefs.svelte'
  import { branchColors, levelStyle, mapTheme } from '../lib/maptheme'

  import type { Snippet } from 'svelte'

  let {
    children,
    onzoom,
    oncontext,
  }: {
    children?: Snippet
    onzoom?: (z: number) => void
    /** Clique direito: num nó (já selecionado) ou no fundo (`id` nulo), em coordenadas da janela. */
    oncontext?: (x: number, y: number, id: number | null) => void
  } = $props()

  let canvas: HTMLCanvasElement
  let stageEl: HTMLDivElement
  let editorEl = $state<HTMLInputElement | undefined>(undefined)


  const view = $state<Viewport>({ x: 0, y: 0, scale: 1 })
  // Cores do tema já resolvidas. Não é estado reativo de propósito: o redesenho lê `colors`, e se ela fosse $state o
  // efeito que a atualiza passaria a depender dela e se dispararia sem parar (era isso que deixava a edição lenta:
  // centenas de redesenhos por tecla, até o Svelte abortar com effect_update_depth_exceeded).
  let colors = readThemeColors()
  let frame = 0

  /** Pede um redesenho no próximo quadro. Várias mudanças no mesmo quadro (snapshot, seleção, hover) viram um só. */
  function scheduleRender() {
    if (frame) return
    frame = requestAnimationFrame(() => {
      frame = 0
      render()
    })
  }
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
    scheduleRender()
  }

  function onContextMenu(e: MouseEvent) {
    e.preventDefault()
    const r = stageEl.getBoundingClientRect()
    const hit = hitTest(doc.snapshot, view, e.clientX, e.clientY, r)
    if (hit !== null) doc.selected = hit
    oncontext?.(e.clientX, e.clientY, hit)
  }

  /** Onde abrir o menu do nó selecionado pelo teclado (tecla de menu, ⇧F10): logo abaixo dele, na janela. */
  export function selectionAnchor(): { x: number; y: number } | null {
    const n = doc.selectedNode
    if (!n) return null
    const r = stageEl.getBoundingClientRect()
    return { x: r.left + (n.x + 12) * view.scale + view.x, y: r.top + (n.y + n.height) * view.scale + view.y + 4 }
  }

  /** Leva a vista até `to`, com uma animação curta (ou na hora). */
  function animateTo(to: { scale: number; x: number; y: number }, animate: boolean) {
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

  /** Enquadra uma caixa do mundo na tela, sem passar de `maxScale`. */
  function frameBox(b: { min_x: number; min_y: number; max_x: number; max_y: number }, maxScale: number, animate: boolean) {
    const r = stageEl.getBoundingClientRect()
    // Margem proporcional: 110 px fixos numa tela de celular deixavam o mapa em 16%.
    const pad = Math.min(110, Math.max(16, Math.min(r.width, r.height) * 0.08))
    const w = b.max_x - b.min_x || 1
    const h = b.max_y - b.min_y || 1
    const target = Math.max(0.12, Math.min((r.width - pad * 2) / w, (r.height - pad * 2) / h, maxScale))
    animateTo(
      {
        scale: target,
        x: r.width / 2 - ((b.min_x + b.max_x) / 2) * target,
        y: r.height / 2 - ((b.min_y + b.max_y) / 2) * target,
      },
      animate,
    )
  }

  /** O mapa inteiro na tela (⇧1). */
  export function fit(animate = true) {
    if (!doc.hasDocument) return
    frameBox(doc.snapshot.bounds, 1.35, animate)
  }

  /** O nó selecionado e o ramo visível abaixo dele na tela (⇧2). */
  export function fitSelection() {
    const sel = doc.selectedNode
    if (!sel) return
    const ids = new Set([sel.id])
    const box = { min_x: sel.x, min_y: sel.y, max_x: sel.x + sel.width, max_y: sel.y + sel.height }
    // Pré-ordem: o pai vem antes dos filhos, então uma passada acha o ramo inteiro.
    for (const n of doc.nodes) {
      if (n.parent === null || !ids.has(n.parent)) continue
      ids.add(n.id)
      box.min_x = Math.min(box.min_x, n.x)
      box.min_y = Math.min(box.min_y, n.y)
      box.max_x = Math.max(box.max_x, n.x + n.width)
      box.max_y = Math.max(box.max_y, n.y + n.height)
    }
    frameBox(box, 2, true)
  }

  /** Tamanho real (⌘0), centrado no nó selecionado, ou no centro da tela se não houver. */
  export function actualSize() {
    const r = stageEl.getBoundingClientRect()
    const sel = doc.selectedNode
    const cx = sel ? sel.x + sel.width / 2 : (r.width / 2 - view.x) / view.scale
    const cy = sel ? sel.y + sel.height / 2 : (r.height / 2 - view.y) / view.scale
    animateTo({ scale: 1, x: r.width / 2 - cx, y: r.height / 2 - cy }, true)
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

  // As cores só mudam com o tema (paleta, claro/escuro, área de trabalho imitada).
  $effect(() => {
    void theme.palette
    void theme.pref
    void theme.simDesktop
    void theme.mode
    colors = readThemeColors()
    scheduleRender()
  })

  // Redesenha quando o que aparece no mapa muda. O desenho em si roda fora do efeito (no quadro seguinte), então o que
  // ele lê não vira dependência.
  $effect(() => {
    void doc.snapshot
    void doc.selected
    void doc.hovered
    void doc.matches
    void prefs.linkStyle
    scheduleRender()
  })

  function onPointerDown(e: PointerEvent) {
    const r = stageEl.getBoundingClientRect()
    // A bolinha de nota abre (ou fecha) a nota formatada ao lado do nó.
    const dot = hitNoteDot(doc.snapshot, view, e.clientX, e.clientY, r)
    if (dot !== null) {
      doc.selected = dot
      notes.toggle(dot)
      return
    }
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
      scheduleRender()
      return
    }
    if (panning) {
      view.x = e.clientX - panning.x
      view.y = e.clientY - panning.y
      scheduleRender()
      return
    }
    const r = stageEl.getBoundingClientRect()
    const hit = hitTest(doc.snapshot, view, e.clientX, e.clientY, r)
    if (hit !== doc.hovered) doc.hovered = hit
    stageEl.classList.toggle('on-dot', hitNoteDot(doc.snapshot, view, e.clientX, e.clientY, r) !== null)
  }

  /** Põe uma nota solta onde a pessoa clicou (coordenadas da janela) e já abre ela para escrever. */
  export async function addNoteAt(clientX: number, clientY: number) {
    const r = stageEl.getBoundingClientRect()
    const x = (clientX - r.left - view.x) / view.scale
    const y = (clientY - r.top - view.y) / view.scale
    const id = await doc.addFreeNote(Math.round(x), Math.round(y))
    if (id !== null) notes.editing = { kind: 'free', id }
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
    // Como no Ideario: o tópico é um card, e abrir é abrir o card (o título se edita no lugar com Espaço ou F2).
    notes.show(hit)
  }

  /**
   * Nós com nota, desenhados como o card do Ideario (HTML e CSS dele) por cima do canvas, na posição e no zoom do mapa.
   * Não recebem o mouse: clicar, arrastar e o menu continuam com o canvas, como em qualquer nó.
   */
  const cardNodes = $derived.by(() => {
    void theme.palette
    void theme.pref
    void theme.simDesktop
    const list = doc.snapshot.nodes.filter((n) => isCard(n))
    if (list.length === 0) return []
    const c = readThemeColors()
    const branch = branchColors(doc.snapshot.nodes, mapTheme(doc.snapshot.style), theme.mode, c.nodeColors, (x) => resolveColor(x, c))
    return list.map((n) => ({ n, fill: cardFill(branch.get(n.id) ?? c.muted, c.surface, theme.mode), blocks: previewFromMarkdown(n.note) }))
  })


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
    if (isCard(n)) {
      // No card, o campo fica sobre o título.
      return {
        left: (n.x + CARD.padX - 4) * view.scale + view.x,
        top: (n.y + CARD.padY - 4) * view.scale + view.y,
        width: (n.width - CARD.padX * 2 + 8) * view.scale,
        height: (CARD.titleSize * CARD.titleLine + 8) * view.scale,
        font: CARD.titleSize * view.scale,
        weight: 600,
      }
    }
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
  /** Tamanho do palco, para os cards de nota não saírem dele. */
  let stage = $state({ w: 0, h: 0 })

  onMount(() => {
    // Retoma a escala da sessão anterior; `fit` a substitui se for a estreia.
    view.scale = prefs.zoom
    render()
    ro = new ResizeObserver(() => {
      stage = { w: stageEl.clientWidth, h: stageEl.clientHeight }
      scheduleRender()
    })
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
  oncontextmenu={onContextMenu}
  role="application"
  aria-label="Mapa mental"
>
  <canvas bind:this={canvas}></canvas>
  {#each cardNodes as c (c.n.id)}
    <div
      class="card map-card"
      style:left="{c.n.x * view.scale + view.x}px"
      style:top="{c.n.y * view.scale + view.y}px"
      style:width="{c.n.width}px"
      style:background={c.fill}
      style:transform="scale({view.scale})"
      aria-hidden="true"
    ><MapCard title={c.n.text} blocks={c.blocks} card={c.n.card} /></div>
  {/each}
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

  {#each doc.snapshot.notes as f (f.id)}
    <NoteCard
      note={{ kind: 'free', id: f.id }}
      text={f.text}
      left={f.x * view.scale + view.x}
      top={f.y * view.scale + view.y}
      width={f.width}
      height={f.height}
      scale={view.scale}
      onsave={(t) => void doc.setFreeNote(f.id, t)}
      onremove={() => void doc.removeFreeNote(f.id)}
      onmove={(dx, dy) => void doc.placeFreeNote(f.id, Math.round(f.x + dx), Math.round(f.y + dy), f.width, f.height)}
      onresize={(w, h) => void doc.placeFreeNote(f.id, f.x, f.y, w, h)}
    />
  {/each}
  {#if notes.opened !== null}
    {@const opened = doc.node(notes.opened)}
    {#if opened}
      {#key opened.id}
        <NodeNote node={opened} autofocus={notes.focus} onclose={() => notes.close()} />
      {/key}
    {/if}
  {/if}

  {@render children?.()}
</div>

<style>
  .stage:global(.on-dot) canvas { cursor: pointer; }
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

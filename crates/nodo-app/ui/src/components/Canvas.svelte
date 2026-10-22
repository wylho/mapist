<script lang="ts">
  // Canvas do mapa: desenha, faz pan/zoom, seleciona e arrasta nós.
  import { onMount, untrack } from 'svelte'
  import { doc } from '../lib/document.svelte'
  import { theme } from '../lib/theme.svelte'
  import { draw, hitKnob, hitTest, readThemeColors, resolveColor, type Viewport } from '../lib/render'
  import { CARD, isCard, summaryOf } from '../lib/cardtext'
  import { attachments } from '../lib/api'
  import { ContextMenu, NoteCard as Card, nearestNoteColor, SEP, t, type MenuEntry, type NoteColor } from '@wylho/atrio'
  import { BookOpen, Trash2 } from '@lucide/svelte'
  import { notes } from '../lib/notes.svelte'
  import NodeNote from './NodeNote.svelte'
  import { prefs } from '../lib/prefs.svelte'
  import { ui } from '../lib/ui.svelte'
  import { groupMenu } from '../lib/mapmenus'
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
  /** Arrastando o grupo: onde o ponteiro começou e as posições de saída de cada tópico, para voltar antes de gravar. */
  let groupDrag: { sx: number; sy: number; origins: Map<number, { x: number; y: number }> } | null = null
  /** Quanto o grupo andou (unidades do mundo): as notas soltas do grupo andam junto na tela. */
  let groupShift = $state<{ dx: number; dy: number } | null>(null)
  /** O retângulo de escolher, em pixels da área do mapa; `add` soma ao grupo que já havia (⇧). */
  let marquee = $state<{ x0: number; y0: number; x1: number; y1: number; add: boolean } | null>(null)
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
    // No grupo, o menu é do grupo: a escolha fica como está.
    if (hit !== null && !(doc.grouped && doc.isPicked(hit))) doc.selected = hit
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
      picked: doc.picked,
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
    void doc.picked
    void doc.hovered
    void doc.matches
    void prefs.linkStyle
    scheduleRender()
  })

  /** Abaixo desta escala o card mostra só o emoji e o título, legíveis (como no Miro): o mapa afastado é visão geral. */
  const FAR = 0.5
  /** Tamanho do título do card afastado, na tela. */
  const FAR_FONT = 12

  /**
   * Onde e como o card fica por cima do canvas (x, y, largura e altura em unidades do mundo). Até 100% ele é montado no
   * tamanho do mapa e encolhido por `transform`: com `zoom`, o WebKit do macOS não deixa a letra ficar menor que ~9 px
   * e o texto estourava a caixa (0.4.1). Acima de 100%, `zoom`: o card é desenhado já ampliado, e o emoji (imagem no
   * macOS) e as fotos ficam nítidos; com `zoom`, `left`/`top` também são ampliados, por isso vão em unidades do mundo.
   */
  function cardStyle(x: number, y: number, w: number, h: number) {
    const s = view.scale
    if (s >= 1) return `left: ${x + view.x / s}px; top: ${y + view.y / s}px; width: ${w}px; zoom: ${s}`
    const far = s < FAR ? `; height: ${h}px; --far-font: ${FAR_FONT / s}px` : ''
    return `left: ${x * s + view.x}px; top: ${y * s + view.y}px; width: ${w}px; transform: scale(${s})${far}`
  }

  // Com o Espaço apertado, a mão aparece: arrastar move o mapa.
  $effect(() => {
    stageEl?.classList.toggle('space', ui.spaceHeld)
  })

  function onPointerDown(e: PointerEvent) {
    const r = stageEl.getBoundingClientRect()
    // Mover o mapa: botão do meio, Espaço apertado, ou o arrasto do fundo quando a preferência é mover.
    if (e.button === 1 || ui.spaceHeld) {
      e.preventDefault()
      startPan(e)
      if (ui.spaceHeld) ui.spacePanned = true
      return
    }
    if (e.button !== 0) return
    const add = e.shiftKey || e.metaKey || e.ctrlKey
    // O botão no começo do ramo recolhe ou expande os filhos.
    const knob = hitKnob(doc.snapshot, view, e.clientX, e.clientY, r)
    if (knob !== null && !add) {
      doc.selected = knob
      void doc.toggleFold()
      return
    }
    const hit = hitTest(doc.snapshot, view, e.clientX, e.clientY, r)
    if (hit !== null) {
      // ⇧/⌘ + clique soma ou tira o tópico do grupo.
      if (add) {
        doc.toggle(hit)
        return
      }
      // Num tópico do grupo, arrastar leva o grupo inteiro.
      if (doc.grouped && doc.isPicked(hit)) {
        doc.focus(hit)
        startGroupDrag(e)
        return
      }
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
    // O fundo: escolher num retângulo (no toque, sempre mover).
    if (e.pointerType !== 'touch' && (prefs.dragBackground === 'select' || e.shiftKey)) {
      const x = e.clientX - r.left
      const y = e.clientY - r.top
      marquee = { x0: x, y0: y, x1: x, y1: y, add }
      return
    }
    startPan(e)
  }

  function startPan(e: PointerEvent) {
    panning = { x: e.clientX - view.x, y: e.clientY - view.y }
    stageEl.classList.add('panning')
  }

  /** Começa a arrastar o grupo (de um tópico ou de uma nota solta do grupo). */
  function startGroupDrag(e: PointerEvent) {
    const origins = new Map<number, { x: number; y: number }>()
    for (const id of doc.selection) {
      const n = doc.node(id)
      if (n) origins.set(id, { x: n.x, y: n.y })
    }
    groupDrag = { sx: e.clientX, sy: e.clientY, origins }
    draggedFar = false
    stageEl.classList.add('moving')
  }

  /** Os tópicos e notas soltas que o retângulo toca, e escolhe eles. */
  function finishMarquee(m: NonNullable<typeof marquee>) {
    const small = Math.abs(m.x1 - m.x0) < 4 && Math.abs(m.y1 - m.y0) < 4
    if (small) {
      // Um clique no fundo desfaz o grupo.
      if (!m.add) doc.clearGroup()
      return
    }
    const wx0 = (Math.min(m.x0, m.x1) - view.x) / view.scale
    const wy0 = (Math.min(m.y0, m.y1) - view.y) / view.scale
    const wx1 = (Math.max(m.x0, m.x1) - view.x) / view.scale
    const wy1 = (Math.max(m.y0, m.y1) - view.y) / view.scale
    const touches = (x: number, y: number, w: number, h: number) => x < wx1 && x + w > wx0 && y < wy1 && y + h > wy0
    const ids = doc.nodes.filter((n) => touches(n.x, n.y, n.width, n.height)).map((n) => n.id)
    const noteIds = doc.snapshot.notes.filter((f) => touches(f.x, f.y, f.width, f.height)).map((f) => f.id)
    doc.pickArea(ids, noteIds, m.add)
  }

  function onPointerMove(e: PointerEvent) {
    if (marquee) {
      const r = stageEl.getBoundingClientRect()
      marquee = { ...marquee, x1: e.clientX - r.left, y1: e.clientY - r.top }
      return
    }
    if (groupDrag) {
      const dx = (e.clientX - groupDrag.sx) / view.scale
      const dy = (e.clientY - groupDrag.sy) / view.scale
      if (Math.abs(dx) * view.scale > 2 || Math.abs(dy) * view.scale > 2) draggedFar = true
      for (const [id, o] of groupDrag.origins) {
        const n = doc.node(id)
        if (n) {
          n.x = o.x + dx
          n.y = o.y + dy
        }
      }
      groupShift = { dx, dy }
      scheduleRender()
      return
    }
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
    // Em cima do botão de recolher, o tópico continua "com o mouse em cima" (senão o botão some antes do clique).
    const knob = hitKnob(doc.snapshot, view, e.clientX, e.clientY, r)
    const hit = hitTest(doc.snapshot, view, e.clientX, e.clientY, r) ?? knob
    if (hit !== doc.hovered) doc.hovered = hit
    stageEl.classList.toggle('on-dot', knob !== null)
  }

  /** Põe uma nota solta onde a pessoa clicou (coordenadas da janela) e já abre ela para escrever. */
  export async function addNoteAt(clientX: number, clientY: number) {
    const r = stageEl.getBoundingClientRect()
    const x = (clientX - r.left - view.x) / view.scale
    const y = (clientY - r.top - view.y) / view.scale
    const id = await doc.addFreeNote(Math.round(x), Math.round(y))
    // A nota nova já abre como card, para escrever.
    if (id !== null) notes.showFree(id, true)
  }

  function onPointerUp() {
    if (marquee) {
      finishMarquee(marquee)
      marquee = null
    }
    if (groupDrag) {
      const g = groupDrag
      const shift = groupShift
      groupDrag = null
      // As posições voltam ao que eram: quem grava o movimento (num passo só) é o núcleo.
      for (const [id, o] of g.origins) {
        const n = doc.node(id)
        if (n) {
          n.x = o.x
          n.y = o.y
        }
      }
      if (draggedFar && shift) void doc.moveGroup(shift.dx, shift.dy).then(() => (groupShift = null))
      else groupShift = null
    }
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

  // Arrastar uma nota solta: o card anda já, e o mapa recebe uma edição só, ao soltar.
  let freeDrag = $state<{ id: number; dx: number; dy: number } | null>(null)

  function startFreeDrag(e: PointerEvent, id: number) {
    // O seletor de status e os botões do card são deles.
    if (e.button !== 0 || (e.target as HTMLElement).closest('button')) return
    // Espaço apertado: o arrasto move o mapa, mesmo começando em cima da nota.
    if (ui.spaceHeld) return
    e.stopPropagation()
    if (e.shiftKey || e.metaKey || e.ctrlKey) {
      doc.toggle(id, true)
      return
    }
    if (doc.pickedNotes.has(id)) {
      startGroupDrag(e)
      const up = () => {
        window.removeEventListener('pointerup', up)
        onPointerUp()
      }
      const move = (ev: PointerEvent) => onPointerMove(ev)
      window.addEventListener('pointermove', move)
      window.addEventListener('pointerup', () => window.removeEventListener('pointermove', move), { once: true })
      window.addEventListener('pointerup', up)
      return
    }
    if (doc.grouped) doc.clearGroup()
    const sx = e.clientX
    const sy = e.clientY
    const el = e.currentTarget as HTMLElement
    const move = (ev: PointerEvent) => {
      freeDrag = { id, dx: (ev.clientX - sx) / view.scale, dy: (ev.clientY - sy) / view.scale }
    }
    const up = () => {
      window.removeEventListener('pointermove', move)
      window.removeEventListener('pointerup', up)
      const d = freeDrag
      const f = doc.freeNote(id)
      if (d && f && Math.hypot(d.dx, d.dy) * view.scale > 3) {
        // A altura gravada é a do card na tela (cresce com o texto): "ver o mapa inteiro" conta com ela.
        const h = Math.ceil(el.getBoundingClientRect().height / view.scale)
        void doc.placeFreeNote(id, Math.round(f.x + d.dx), Math.round(f.y + d.dy), f.width, h).then(() => (freeDrag = null))
      } else freeDrag = null
    }
    window.addEventListener('pointermove', move)
    window.addEventListener('pointerup', up)
  }

  /** O resumo do card de uma nota solta, como o de um tópico (mesmo `NoteCard`, cor de nota dela). */
  function freeSummary(f: (typeof doc.snapshot.notes)[number]) {
    return summaryOf({ text: f.title, card: f.card, view: f.view }, (f.color ?? 'none') as NoteColor)
  }

  function freeMenu(id: number): MenuEntry[] {
    // Nota solta do grupo: o menu é o do grupo.
    if (doc.pickedNotes.has(id) && doc.grouped) return groupMenu()
    return [
      { label: t('Abrir nota'), icon: BookOpen, onSelect: () => notes.showFree(id) },
      SEP,
      { label: t('Apagar nota'), icon: Trash2, danger: true, onSelect: () => void doc.removeFreeNote(id) },
    ]
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
    // O card padrão do Atrio, na cor de nota mais próxima da cor do ramo (as cores dos cards são as do Atrio).
    return list.map((n) => ({ n, summary: summaryOf(n, nearestNoteColor(branch.get(n.id) ?? null)) }))
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

  /** A aba à vista: ao trocar, guarda o zoom e a posição dela e volta aos da outra (ou enquadra a que nunca foi vista). */
  let shownTab: number | null | undefined = undefined
  /** O enquadramento de uma aba recém-aberta ainda por acontecer (dois quadros depois). */
  let pendingFit: number | null = null

  $effect(() => {
    const id = doc.snapshot.active
    const has = doc.hasDocument
    untrack(() => {
      if (id === shownTab || !has) return
      // Saiu antes de a aba ser enquadrada: não guarda uma posição que não é dela (enquadra quando ela voltar).
      if (pendingFit !== null) {
        cancelAnimationFrame(pendingFit)
        pendingFit = null
      } else if (shownTab != null) doc.views.set(shownTab, { x: view.x, y: view.y, scale: view.scale })
      shownTab = id
      const saved = id !== null ? doc.views.get(id) : undefined
      if (saved) {
        Object.assign(view, saved)
        scheduleRender()
        return
      }
      // Dois quadros: o primeiro deixa o ResizeObserver medir o palco, o segundo
      // enquadra sobre um tamanho já correto. Com um só, o fit usava o rect
      // anterior e o mapa ficava fora de escala.
      pendingFit = requestAnimationFrame(() => {
        pendingFit = requestAnimationFrame(() => {
          pendingFit = null
          fit(false)
        })
      })
    })
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
  aria-label={t('Mapa mental')}
>
  <canvas bind:this={canvas}></canvas>
  {#each cardNodes as c (c.n.id)}
    <Card
      class="map-card{view.scale < FAR ? ' far' : ''}"
      card={c.summary}
      thumb={(h) => attachments.thumb(h)}
      style={cardStyle(c.n.x, c.n.y, c.n.width, c.n.height)}
      tabindex={-1}
      onstatus={(s) => void doc.setStatus(c.n.id, s)}
    />
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
      aria-label={t('Texto do nó')}
    />
  {/if}

  {#each doc.snapshot.notes as f (f.id)}
    {@const moved = freeDrag?.id === f.id ? freeDrag : null}
    <ContextMenu items={() => freeMenu(f.id)}>
      {#snippet children(trigger)}
        <Card
          {...trigger}
          class="map-card free-card{doc.pickedNotes.has(f.id) ? ' picked' : ''}{view.scale < FAR ? ' far' : ''}"
          card={freeSummary(f)}
          thumb={(h) => attachments.thumb(h)}
          style={cardStyle(
            f.x + (moved?.dx ?? 0) + (doc.pickedNotes.has(f.id) ? (groupShift?.dx ?? 0) : 0),
            f.y + (moved?.dy ?? 0) + (doc.pickedNotes.has(f.id) ? (groupShift?.dy ?? 0) : 0),
            f.width,
            f.height,
          )}
          data-free-note={f.id}
          onpointerdown={(e) => startFreeDrag(e, f.id)}
          ondblclick={(e) => {
            e.stopPropagation()
            notes.showFree(f.id)
          }}
          onkeydown={(e) => e.key === 'Enter' && notes.showFree(f.id)}
          onstatus={(st) => void doc.setFreeStatus(f.id, st)}
        />
      {/snippet}
    </ContextMenu>
  {/each}
  {#if notes.opened !== null && doc.node(notes.opened)}
    {#key notes.opened}
      <NodeNote target={{ kind: 'node', id: notes.opened }} autofocus={notes.focus} onclose={() => notes.close()} />
    {/key}
  {:else if notes.openedFree !== null && doc.freeNote(notes.openedFree)}
    {#key notes.openedFree}
      <NodeNote target={{ kind: 'free', id: notes.openedFree }} autofocus={notes.focus} onclose={() => notes.closeFree()} />
    {/key}
  {/if}

  {#if marquee}
    <div
      class="marquee"
      style:left="{Math.min(marquee.x0, marquee.x1)}px"
      style:top="{Math.min(marquee.y0, marquee.y1)}px"
      style:width="{Math.abs(marquee.x1 - marquee.x0)}px"
      style:height="{Math.abs(marquee.y1 - marquee.y0)}px"
    ></div>
  {/if}
  {@render children?.()}
</div>

<style>
  .stage:global(.on-dot) canvas { cursor: pointer; }
  .stage:global(.space) canvas { cursor: grab; }
  /* O retângulo de escolher, como no Figma e no Excalidraw: borda e fundo leves no destaque. */
  .marquee {
    position: absolute;
    z-index: 6;
    pointer-events: none;
    border: 1px solid var(--accent);
    background: color-mix(in srgb, var(--accent) 10%, transparent);
    border-radius: 2px;
  }
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

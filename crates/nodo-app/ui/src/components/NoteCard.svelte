<script lang="ts">
  // Card de nota no mapa, no desenho do card de nota do Ideario, translúcido para parecer que flutua. Mostra o
  // Markdown formatado; um clique no texto edita ali mesmo, e o que se escreve é salvo sozinho. Serve à nota de um nó
  // (presa ao lado dele, com o nome do nó numa pílula) e à nota solta no fundo do mapa (arrasta pela barra e acompanha
  // o zoom, porque faz parte do mapa). Não bloqueia o mapa: só o próprio card recebe o mouse.
  import { untrack } from 'svelte'
  import { Check, GripHorizontal, Trash2, X } from '@lucide/svelte'
  import { notes, type NoteRef } from '../lib/notes.svelte'
  import { openLink, renderMarkdown } from '../lib/markdown'

  let {
    note,
    text,
    left,
    top,
    width,
    height,
    scale = 1,
    label = null,
    color = null,
    onsave,
    onclose,
    onremove,
    onmove,
    onresize,
  }: {
    note: NoteRef
    text: string
    /** Canto do card na tela, em px. */
    left: number
    top: number
    /** Tamanho em unidades do card (px na nota de nó, mundo na nota solta, que a `scale` aumenta ou diminui). */
    width: number
    height: number
    scale?: number
    /** Nome do nó a que a nota pertence (nota solta não tem). */
    label?: string | null
    color?: string | null
    onsave: (text: string) => void
    onclose?: () => void
    onremove?: () => void
    /** Arrastou a barra: deslocamento em unidades do card. */
    onmove?: (dx: number, dy: number) => void
    onresize: (width: number, height: number) => void
  } = $props()

  const editing = $derived(notes.isEditing(note.kind, note.id))
  const html = $derived(text.trim() ? renderMarkdown(text) : '')

  let draft = $state('')
  let area = $state<HTMLTextAreaElement | undefined>(undefined)
  let body = $state<HTMLElement | undefined>(undefined)
  let el = $state<HTMLElement | undefined>(undefined)
  let timer: ReturnType<typeof setTimeout> | undefined

  // Arrastar a barra ou o canto: o card anda já, e o mapa recebe uma edição só, ao soltar.
  let shift = $state({ x: 0, y: 0 })
  let grow = $state<{ w: number; h: number } | null>(null)

  // Entrar em edição: o rascunho começa do texto salvo e o cursor vai para o fim.
  $effect(() => {
    if (!editing) return
    // Só ao entrar: o texto salvo voltando do núcleo enquanto se digita não pode apagar o que veio depois.
    draft = untrack(() => text)
    requestAnimationFrame(() => {
      if (!area) return
      area.focus()
      area.setSelectionRange(area.value.length, area.value.length)
    })
  })

  function edit() {
    notes.editing = { ...note }
  }

  function flush() {
    clearTimeout(timer)
    timer = undefined
    if (draft !== text) onsave(draft)
  }

  function done() {
    flush()
    if (notes.isEditing(note.kind, note.id)) notes.editing = null
  }

  function onInput() {
    clearTimeout(timer)
    timer = setTimeout(flush, 450)
  }

  function onKey(e: KeyboardEvent) {
    if (e.key === 'Escape' || (e.key === 'Enter' && (e.metaKey || e.ctrlKey))) {
      e.preventDefault()
      e.stopPropagation()
      done()
    }
  }

  function onBlur(e: FocusEvent) {
    // Ir para um botão do próprio card não encerra a edição; clicar no mapa, sim.
    if (el && e.relatedTarget instanceof Node && el.contains(e.relatedTarget)) return
    done()
  }

  function onBodyClick(e: MouseEvent) {
    const a = (e.target as HTMLElement).closest('a')
    if (a) {
      e.preventDefault()
      void openLink(a.getAttribute('href') ?? '')
      return
    }
    // Clicar para escrever, mas não quando a pessoa só selecionou um trecho para copiar.
    if (!window.getSelection()?.toString()) edit()
  }

  function track(e: PointerEvent, move: (dx: number, dy: number) => void, end: () => void) {
    if (e.button !== 0) return
    e.preventDefault()
    const sx = e.clientX
    const sy = e.clientY
    const target = e.currentTarget as HTMLElement
    target.setPointerCapture(e.pointerId)
    const onMove = (ev: PointerEvent) => move(ev.clientX - sx, ev.clientY - sy)
    const onUp = () => {
      target.removeEventListener('pointermove', onMove)
      target.removeEventListener('pointerup', onUp)
      target.removeEventListener('pointercancel', onUp)
      end()
    }
    target.addEventListener('pointermove', onMove)
    target.addEventListener('pointerup', onUp)
    target.addEventListener('pointercancel', onUp)
  }

  function startMove(e: PointerEvent) {
    if (!onmove || (e.target as HTMLElement).closest('button')) return
    track(
      e,
      (dx, dy) => (shift = { x: dx, y: dy }),
      () => {
        const { x, y } = shift
        if (Math.hypot(x, y) > 2) onmove?.(x / scale, y / scale)
        shift = { x: 0, y: 0 }
      },
    )
  }

  function startResize(e: PointerEvent) {
    track(
      e,
      (dx, dy) => (grow = { w: Math.max(200, width + dx / scale), h: Math.max(120, height + dy / scale) }),
      () => {
        if (grow && (grow.w !== width || grow.h !== height)) onresize(Math.round(grow.w), Math.round(grow.h))
        grow = null
      },
    )
  }

  // A roda rola o texto quando há o que rolar (ou se está escrevendo); senão, passa para o mapa (pan e zoom).
  function onWheel(e: WheelEvent) {
    const scrollable = body && body.scrollHeight > body.clientHeight + 1
    if (editing || (scrollable && !e.ctrlKey && !e.metaKey)) e.stopPropagation()
  }
</script>

<section
  bind:this={el}
  class="ncard"
  class:free={note.kind === 'free'}
  class:editing
  style:left="{left + shift.x}px"
  style:top="{top + shift.y}px"
  style:width="{grow?.w ?? width}px"
  style:height="{grow?.h ?? height}px"
  style:transform={scale === 1 ? null : `scale(${scale})`}
  aria-label={label ? `Nota de ${label}` : 'Nota'}
  onpointerdown={(e) => e.stopPropagation()}
  onwheel={onWheel}
  ondblclick={(e) => e.stopPropagation()}
  oncontextmenu={(e) => e.stopPropagation()}
>
  <!-- svelte-ignore a11y_no_static_element_interactions -- a barra arrasta com o mouse; pelo teclado a nota se edita no corpo -->
  <header class="ncard-bar" class:grab={!!onmove} onpointerdown={startMove}>
    {#if label !== null}
      <span class="pill ncard-node" title={label}><i class="dot" style:background={color ?? 'var(--muted)'}></i>{label}</span>
    {:else}
      <span class="ncard-grip" aria-hidden="true"><GripHorizontal size={14} /></span>
    {/if}
    <span class="ncard-acts">
      {#if editing}
        <button class="icon-btn sm" aria-label="Concluir" title="Concluir (Esc)" onclick={done}><Check size={15} /></button>
      {/if}
      {#if onremove}
        <button class="icon-btn sm" aria-label="Apagar a nota" title="Apagar a nota" onclick={onremove}><Trash2 size={14} /></button>
      {/if}
      {#if onclose}
        <button class="icon-btn sm" aria-label="Fechar a nota" title="Fechar" onclick={onclose}><X size={15} /></button>
      {/if}
    </span>
  </header>
  {#if editing}
    <textarea
      bind:this={area}
      bind:value={draft}
      class="ncard-text"
      placeholder="Escreva a nota… (aceita Markdown)"
      aria-label="Texto da nota"
      oninput={onInput}
      onkeydown={onKey}
      onblur={onBlur}
    ></textarea>
  {:else}
    <!-- eslint-disable-next-line svelte/no-at-html-tags -- HTML limpo pelo DOMPurify em renderMarkdown -->
    <div bind:this={body} class="ncard-body md" role="button" tabindex="0" title="Clique para editar"
      onclick={onBodyClick} onkeydown={(e) => e.key === 'Enter' && (e.preventDefault(), edit())}
    >{#if html}{@html html}{:else}<p class="ncard-empty">Escreva a nota…</p>{/if}</div>
  {/if}
  <span class="ncard-resize" aria-hidden="true" onpointerdown={startResize}></span>
</section>

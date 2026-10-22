<script lang="ts">
  // Tema do mapa, como no XMind: escolhe a forma de cada nível e as cores dos ramos. Fica no arquivo e desfaz como
  // qualquer edição. Cada opção é um mapinha desenhado pelo próprio renderizador, então a prévia é o que vai sair.
  import { Dialog } from 'bits-ui'
  import { ToggleGroup } from 'bits-ui'
  import { WandSparkles, X } from '@lucide/svelte'
  import { SPACINGS, STRUCTURES, sketch, spacingOf, structureOf } from '../lib/structures'
  import { tick } from 'svelte'
  import { doc } from '../lib/document.svelte'
  import { theme } from '../lib/theme.svelte'
  import { prefs } from '../lib/prefs.svelte'
  import { MAP_THEMES, MAP_THEME_IDS, mapTheme, type MapThemeId } from '../lib/maptheme'
  import { measureNode } from '../lib/measure'
  import { draw, readThemeColors } from '../lib/render'
  import type { FlatNode, Snapshot } from '../lib/api'

  let { open = $bindable(false) }: { open?: boolean } = $props()

  const current = $derived(mapTheme(doc.snapshot.style).id)
  const structure = $derived(structureOf(doc.snapshot.structure))
  const spacing = $derived(spacingOf(doc.snapshot.spacing))
  const canvases: Record<string, HTMLCanvasElement | undefined> = $state({})

  /** Um mapa pequeno (raiz, três ramos, dois sub-ramos) posicionado à mão, medido no tema que vai mostrar. */
  function sample(id: MapThemeId): Snapshot {
    const t = MAP_THEMES[id]
    const make = (id: number, text: string, depth: number, parent: number | null): FlatNode => ({
      id, text, note: '', side: 'right', depth, x: 0, y: 0, width: 0, height: 0, color: null, folded: false, card: {},
      child_ids: [], parent, has_children: false,
    })
    const nodes = [
      make(0, 'Ideia', 0, null),
      make(1, 'Ramo', 1, 0),
      make(2, 'Ramo', 1, 0),
      make(3, 'Ramo', 1, 0),
      make(4, 'Detalhe', 2, 1),
      make(5, 'Detalhe', 2, 1),
    ]
    for (const n of nodes) Object.assign(n, measureNode(n, t))
    const [root, a, b, c, a1, a2] = nodes
    root.x = 0
    root.y = 70 - root.height / 2
    const col1 = root.width + 46
    const ys = [18, 70, 122]
    ;[a, b, c].forEach((n, i) => {
      n.x = col1
      n.y = ys[i] - n.height / 2
    })
    const col2 = col1 + Math.max(a.width, b.width, c.width) + 40
    a1.x = col2
    a1.y = 0 - a1.height / 2 + 2
    a2.x = col2
    a2.y = 36 - a2.height / 2
    const maxX = Math.max(...nodes.map((n) => n.x + n.width))
    return {
      nodes, connections: [], notes: [], dirty: false, path: null, title: '', can_undo: false, can_redo: false, stale: false,
      style: id, structure: null, spacing: null, bounds: { min_x: 0, min_y: -20, max_x: maxX, max_y: 140 }, tabs: [], active: null,
    }
  }

  function paint() {
    const colors = readThemeColors()
    const dpr = window.devicePixelRatio || 1
    for (const id of MAP_THEME_IDS) {
      const c = canvases[id]
      if (!c) continue
      const w = c.clientWidth
      const h = c.clientHeight
      c.width = Math.round(w * dpr)
      c.height = Math.round(h * dpr)
      const snap = sample(id)
      const b = snap.bounds
      const scale = Math.min((w - 20) / (b.max_x - b.min_x), (h - 16) / (b.max_y - b.min_y))
      const view = { scale, x: (w - (b.max_x - b.min_x) * scale) / 2 - b.min_x * scale, y: (h - (b.max_y - b.min_y) * scale) / 2 - b.min_y * scale }
      draw({
        snapshot: snap, colors, view, selected: null, hovered: null, matches: new Set(), width: w, height: h, dpr,
        linkStyle: prefs.linkStyle, mode: theme.mode, ...({ ctx: c.getContext('2d') } as Record<string, unknown>),
      } as never)
    }
  }

  $effect(() => {
    void theme.palette
    void theme.mode
    void prefs.linkStyle
    if (open) tick().then(() => requestAnimationFrame(paint))
  })

  function pick(id: MapThemeId) {
    // O Clássico é o padrão: sem atributo no arquivo, que fica igual ao do Minder.
    if (id !== current) void doc.setMapStyle(id === 'classico' ? null : id)
  }
</script>

<Dialog.Root bind:open>
  <Dialog.Portal>
    <Dialog.Overlay class="overlay" />
    <Dialog.Content class="sheet" aria-describedby={undefined}>
      <div class="sheet-head">
        <Dialog.Title class="sheet-title">Mapa</Dialog.Title>
        <Dialog.Close class="icon-btn" aria-label="Fechar"><X size={20} /></Dialog.Close>
      </div>
      <div class="sheet-scroll">
        <section class="set-group">
          <h3>Estrutura</h3>
          <div class="structure-grid" role="radiogroup" aria-label="Estrutura do mapa">
            {#each STRUCTURES as st (st.id)}
              {@const sk = sketch(st.id)}
              <button class="theme-tile structure-tile" role="radio" aria-checked={structure === st.id}
                onclick={() => structure !== st.id && doc.setStructure(st.id)}>
                <svg class="structure-sketch" viewBox="0 0 96 56" aria-hidden="true">
                  {#each sk.lines as d (d)}<path {d} />{/each}
                  {#each sk.boxes as [x, y, w, h], i (i)}<rect {x} {y} width={w} height={h} rx="3" class:root={i === 0} />{/each}
                </svg>
                <span class="theme-name">{st.label}</span>
                <small>{st.hint}</small>
              </button>
            {/each}
          </div>
          <div class="set-row">
            <span class="grow"><b>Espaçamento</b><small>Quanto espaço fica entre os nós.</small></span>
            <ToggleGroup.Root type="single" value={spacing} onValueChange={(v) => v && v !== spacing && doc.setSpacing(v)} class="seg tight" aria-label="Espaçamento">
              {#each SPACINGS as sp (sp.id)}<ToggleGroup.Item value={sp.id} class="seg-item">{sp.label}</ToggleGroup.Item>{/each}
            </ToggleGroup.Root>
          </div>
          <div class="set-row">
            <span class="grow"><b>Arrumar posições</b><small>Desfaz os arrastos à mão e põe tudo no lugar da estrutura.</small></span>
            <button class="btn soft sm" onclick={() => doc.relayout()}><WandSparkles size={15} />Arrumar</button>
          </div>
        </section>

        <h3 class="sheet-sub">Tema</h3>
        <p class="map-theme-note">Fica salvo neste mapa. As cores dos ramos saem do tema; uma cor escolhida no painel continua valendo.</p>
        <div class="map-theme-grid" role="radiogroup" aria-label="Tema do mapa">
          {#each MAP_THEME_IDS as id (id)}
            {@const t = MAP_THEMES[id]}
            <button class="theme-tile map-theme-tile" role="radio" aria-checked={current === id} onclick={() => pick(id)}>
              <canvas class="map-theme-preview" bind:this={canvases[id]} aria-hidden="true"></canvas>
              <span class="theme-name">{t.label}</span>
              <small>{t.hint}</small>
            </button>
          {/each}
        </div>
      </div>
    </Dialog.Content>
  </Dialog.Portal>
</Dialog.Root>

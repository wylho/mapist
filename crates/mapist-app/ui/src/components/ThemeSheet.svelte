<script lang="ts">
  // Folha de temas: paleta, área de trabalho imitada e aparência.
  // Mesma estrutura do theme-grid do Ideario.
  import { theme } from '../lib/theme.svelte'
  import { DESKTOPS, PALETTES, systemColors, type Colors, type DesktopId, type PaletteId } from '../lib/palettes'

  let open = $state(false)

  const paletteIds = Object.keys(PALETTES) as Exclude<PaletteId, 'system'>[]
  const desktopIds = Object.keys(DESKTOPS) as DesktopId[]

  function colorsFor(id: PaletteId): Colors {
    return id === 'system'
      ? systemColors(theme.simDesktop, theme.mode, theme.systemAccent)
      : PALETTES[id][theme.mode]
  }

  function close() {
    open = false
  }

  function onKey(e: KeyboardEvent) {
    if (e.key === 'Escape' && open) close()
  }
</script>

<svelte:window onkeydown={onKey} />

<button class="theme-toggle" title="Tema" onclick={() => (open = true)}>
  <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round">
    <circle cx="12" cy="12" r="9" />
    <path d="M12 3a9 9 0 0 0 0 18 4.5 4.5 0 0 1 0-9 4.5 4.5 0 0 0 0-9" />
  </svg>
  <span>{theme.label}</span>
</button>

{#if open}
  <div
    class="overlay"
    role="presentation"
    onclick={close}
  ></div>

  <div class="drawer" role="dialog" aria-label="Tema">
    <div class="drawer-head">
      <span class="brand">Tema</span>
      <button class="icon-btn sm" title="Fechar" onclick={close}>
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round">
          <path d="M18 6 6 18M6 6l12 12" />
        </svg>
      </button>
    </div>

    <div class="drawer-scroll">
      <div class="theme-group-label">Paleta</div>
      <div class="theme-grid" role="radiogroup" aria-label="Tema">
        {#each [{ id: 'system' as PaletteId, label: 'Sistema', hint: 'Segue a área de trabalho' }, ...paletteIds.map((id) => ({ id: id as PaletteId, label: PALETTES[id].label, hint: PALETTES[id].hint }))] as t (t.id)}
          {@const c = colorsFor(t.id)}
          <button
            class="theme-tile"
            role="radio"
            aria-checked={theme.palette === t.id}
            onclick={() => theme.setPalette(t.id)}
          >
            <span class="theme-swatch" style:background={c.bg}>
              <span class="theme-card" style:background={c.surface} style:border-color={c.line}></span>
              <span class="theme-dots">
                {#each c.notes.slice(0, 4) as n (n)}
                  <i style:background={n}></i>
                {/each}
              </span>
            </span>
            <span class="theme-name">{t.label}</span>
            <span class="theme-hint">{t.hint}</span>
          </button>
        {/each}
      </div>

      {#if theme.palette === 'system'}
        <div class="theme-group-label">Imitar área de trabalho</div>
        <div class="theme-grid" role="radiogroup" aria-label="Área de trabalho">
          {#each desktopIds as id (id)}
            {@const c = DESKTOPS[id][theme.mode]}
            <button
              class="theme-tile"
              role="radio"
              aria-checked={theme.simDesktop === id}
              onclick={() => theme.setSimDesktop(id)}
            >
              <span class="theme-swatch" style:background={c.bg}>
                <span class="theme-card" style:background={c.surface} style:border-color={c.line}></span>
                <span class="theme-dots">
                  {#each c.notes.slice(0, 4) as n (n)}
                    <i style:background={n}></i>
                  {/each}
                </span>
              </span>
              <span class="theme-name">{DESKTOPS[id].label}</span>
              <span class="theme-hint">Destaque {c.accent}</span>
            </button>
          {/each}
        </div>
      {/if}

      <div class="theme-group-label">Aparência</div>
      <div class="seg tight" role="radiogroup" aria-label="Aparência">
        {#each [{ v: 'system', l: 'Sistema' }, { v: 'light', l: 'Claro' }, { v: 'dark', l: 'Escuro' }] as m (m.v)}
          <button
            class="seg-item"
            role="radio"
            data-state={theme.pref === m.v ? 'on' : 'off'}
            aria-checked={theme.pref === m.v}
            onclick={() => theme.set(m.v as 'system' | 'light' | 'dark')}>{m.l}</button
          >
        {/each}
      </div>
    </div>
  </div>
{/if}

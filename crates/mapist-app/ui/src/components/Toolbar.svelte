<script lang="ts">
  // Barra superior: marca, ações de arquivo, ações de nó, busca e undo/redo.
  import { doc } from '../lib/document.svelte'
  import { theme } from '../lib/theme.svelte'

  let search = $state('')
  let searchTimer: ReturnType<typeof setTimeout> | undefined

  function onSearch(e: Event) {
    search = (e.target as HTMLInputElement).value
    clearTimeout(searchTimer)
    searchTimer = setTimeout(() => doc.search(search), 160)
  }

  async function doOpen() {
    // O diálogo nativo é do plugin-dialog; sem ele, cai no prompt do navegador.
    try {
      const { open } = await import('@tauri-apps/plugin-dialog')
      const picked = await open({
        multiple: false,
        filters: [{ name: 'Mapa mental', extensions: ['minder'] }],
      })
      if (typeof picked === 'string') await doc.open(picked)
    } catch {
      const path = window.prompt('Caminho do arquivo .minder')
      if (path) await doc.open(path)
    }
  }

  async function doSave() {
    if (doc.snapshot.path) {
      await doc.save()
      return
    }
    try {
      const { save } = await import('@tauri-apps/plugin-dialog')
      const picked = await save({
        defaultPath: 'mapa.minder',
        filters: [{ name: 'Mapa mental', extensions: ['minder'] }],
      })
      if (typeof picked === 'string') await doc.save(picked)
    } catch {
      const path = window.prompt('Salvar como (caminho)')
      if (path) await doc.save(path)
    }
  }
</script>

<header class="mapbar">
  <div class="brand">
    <div class="brand-mark">
      <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round">
        <circle cx="12" cy="12" r="3" />
        <path d="M12 9V4M12 15v5M9 12H4M15 12h5" />
      </svg>
    </div>
    <span class="brand-name">Mapist</span>
  </div>

  <div class="top-left">
    <button class="btn" title="Novo mapa (⌘N)" onclick={() => doc.create('Tópico central')}>
      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><path d="M12 5v14M5 12h14" /></svg>
      Novo
    </button>
    <button class="btn" title="Abrir (⌘O)" onclick={doOpen}>
      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M3 7a2 2 0 0 1 2-2h4l2 2h8a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z" /></svg>
      Abrir
    </button>
    <button class="btn" title="Salvar (⌘S)" onclick={doSave}>
      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M19 21H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h11l5 5v11a2 2 0 0 1-2 2z" /><path d="M17 21v-8H7v8M7 3v5h8" /></svg>
      Salvar
    </button>

    <div style="width:1px;height:24px;background:var(--line);margin:0 6px"></div>

    <button class="btn primary" title="Adicionar filho (Tab)" onclick={() => doc.addChild()}>
      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><path d="M12 5v14M5 12h14" /></svg>
      Filho
    </button>
    <button class="icon-btn" title="Excluir nó (⌫)" onclick={() => doc.remove()}>
      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M3 6h18M8 6V4a1 1 0 0 1 1-1h6a1 1 0 0 1 1 1v2M19 6l-1 14a2 2 0 0 1-2 2H8a2 2 0 0 1-2-2L5 6" /></svg>
    </button>
    <button class="icon-btn" title="Dobrar ramo (Space)" onclick={() => doc.toggleFold()}>
      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><circle cx="12" cy="12" r="3" /><path d="M12 3v4M12 17v4M3 12h4M17 12h4" /></svg>
    </button>
  </div>

  <div class="top-actions">
    <label class="search">
      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><circle cx="11" cy="11" r="7" /><path d="m20 20-3.5-3.5" /></svg>
      <input type="search" placeholder="Buscar nós…" value={search} oninput={onSearch} />
    </label>
    <button class="icon-btn" title="Desfazer (⌘Z)" disabled={!doc.snapshot.can_undo} onclick={() => doc.undo()}>
      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M3 7v6h6" /><path d="M3.5 13a9 9 0 1 0 2.1-9.4L3 7" /></svg>
    </button>
    <button class="icon-btn" title="Refazer (⌘⇧Z)" disabled={!doc.snapshot.can_redo} onclick={() => doc.redo()}>
      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M21 7v6h-6" /><path d="M20.5 13a9 9 0 1 1-2.1-9.4L21 7" /></svg>
    </button>
    <button class="icon-btn" title={theme.mode === 'dark' ? 'Modo claro' : 'Modo escuro'} onclick={() => theme.toggleMode()}>
      {#if theme.mode === 'dark'}
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><path d="M21 12.8A9 9 0 1 1 11.2 3a7 7 0 0 0 9.8 9.8z" /></svg>
      {:else}
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><circle cx="12" cy="12" r="4" /><path d="M12 2v2M12 20v2M4.9 4.9l1.4 1.4M17.7 17.7l1.4 1.4M2 12h2M20 12h2M4.9 19.1l1.4-1.4M17.7 6.3l1.4-1.4" /></svg>
      {/if}
    </button>
  </div>
</header>

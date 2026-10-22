<script lang="ts">
  // Barra superior com a estrutura da TopBar do Ideario (o padrão dos nossos apps):
  // à esquerda, o que é do documento (arquivo e título); à direita, ancorado na ponta, o que está sempre lá
  // (busca, painel, configurações) e, à esquerda disso, as ações do nó selecionado, que só aparecem com seleção.
  import { tick, untrack } from 'svelte'
  import {
    ChevronsDownUp,
    ChevronsUpDown,
    FilePlus2,
    FolderOpen,
    Palette,
    Plus,
    Redo2,
    Save,
    Search,
    Settings,
    Share,
    Trash2,
    Undo2,
    X,
  } from '@lucide/svelte'
  import { t } from '@wylho/atrio'
  import { doc } from '../lib/document.svelte'
  import { prefs } from '../lib/prefs.svelte'
  import { ui } from '../lib/ui.svelte'
  import MapThemeSheet from './MapThemeSheet.svelte'


  const mod = /Mac/.test(navigator.platform) ? '⌘' : 'Ctrl+'
  const overlayTitlebar = document.documentElement.dataset.titlebar === 'overlay'
  const sel = $derived(doc.selectedNode)
  const title = $derived(doc.snapshot.title || t('Sem título'))

  let query = $state(prefs.search)
  let timer: ReturnType<typeof setTimeout> | undefined
  let input: HTMLInputElement | undefined = $state()
  const searchShown = $derived(ui.searchOpen || !!query)

  function setQuery(q: string) {
    query = q
    prefs.setSearch(q)
    clearTimeout(timer)
    timer = setTimeout(() => doc.search(q), 160)
  }

  $effect(() => {
    if (ui.searchOpen) tick().then(() => input?.focus())
  })

  // A busca da última sessão volta marcada no mapa.
  $effect(() => {
    if (doc.hasDocument) untrack(() => query && void doc.search(query))
  })
</script>

<!-- macOS: a barra é também a barra de título (transparente) e arrasta a janela, como a do Ideario. -->
<header class="topbar appbar" data-tauri-drag-region={overlayTitlebar ? 'deep' : undefined}>
  <div class="top-left">
    <button class="icon-btn roomy" aria-label={t('Novo mapa')} title="{t('Novo mapa')} ({mod}N)" onclick={() => doc.newMap()}><FilePlus2 size={20} /></button>
    <button class="icon-btn roomy" aria-label={t('Abrir')} title="{t('Abrir')} ({mod}O)" onclick={() => doc.openDialog()}><FolderOpen size={20} /></button>
    <button class="icon-btn" aria-label={t('Salvar')} title="{t('Salvar')} ({mod}S)" onclick={() => doc.saveDialog()}><Save size={20} /></button>
    <button class="icon-btn roomy" aria-label={t('Exportar')} title="{t('Exportar')} ({mod}⇧E)" onclick={() => (ui.exportOpen = true)}><Share size={19} /></button>
    <button class="icon-btn" aria-label={t('Estrutura e tema do mapa')} title={t('Estrutura e tema do mapa')} onclick={() => (ui.themeOpen = true)}><Palette size={20} /></button>
    <span class="doc-title" title={doc.snapshot.path ?? title}>
      <span class="grow">{title}</span>
      {#if doc.snapshot.dirty}<i class="dirty" aria-label={t('Alterações não salvas')} title={t('Alterações não salvas')}></i>{/if}
    </span>
  </div>

  <div class="top-actions">
    {#if sel}
      <div class="filter-group" role="group" aria-label={t('Nó selecionado')}>
        <button class="icon-btn" aria-label={t('Adicionar filho')} title="{t('Adicionar filho')} (Tab)" onclick={() => doc.addChild()}><Plus size={20} /></button>
        {#if sel.has_children}
          <button class="icon-btn" aria-label={sel.folded ? t('Expandir ramo') : t('Recolher ramo')} title="{sel.folded ? t('Expandir ramo') : t('Recolher ramo')} ({mod}/)" onclick={() => doc.toggleFold()}>
            {#if sel.folded}<ChevronsUpDown size={19} />{:else}<ChevronsDownUp size={19} />{/if}
          </button>
        {/if}
        {#if sel.parent !== null}
          <button class="icon-btn" aria-label={t('Excluir nó')} title="{t('Excluir nó')} (Delete)" onclick={() => doc.remove()}><Trash2 size={19} /></button>
        {/if}
      </div>
    {/if}
    {#if doc.snapshot.can_undo || doc.snapshot.can_redo}
      <div class="filter-group" role="group" aria-label={t('Histórico')}>
        <button class="icon-btn" aria-label={t('Desfazer')} title="{t('Desfazer')} ({mod}Z)" disabled={!doc.snapshot.can_undo} onclick={() => doc.undo()}><Undo2 size={19} /></button>
        <button class="icon-btn" aria-label={t('Refazer')} title="{t('Refazer')} ({mod}⇧Z)" disabled={!doc.snapshot.can_redo} onclick={() => doc.redo()}><Redo2 size={19} /></button>
      </div>
    {/if}
    {#if searchShown}
      <label class="search inline">
        <Search size={16} aria-hidden="true" />
        <input
          id="busca"
          bind:this={input}
          value={query}
          placeholder={t('Buscar no mapa')}
          autocomplete="off"
          spellcheck="false"
          oninput={(e) => setQuery(e.currentTarget.value)}
          onblur={() => { if (!query) ui.searchOpen = false }}
          onkeydown={(e) => {
            if (e.key === 'Escape') {
              if (query) setQuery('')
              else e.currentTarget.blur()
            }
          }}
        />
        {#if query}
          <button class="icon-btn sm" aria-label={t('Limpar busca')} onclick={() => { setQuery(''); input?.focus() }}><X size={14} /></button>
        {/if}
      </label>
    {:else}
      <button class="icon-btn" aria-label={t('Buscar')} title="{t('Buscar')} ({mod}F)" onclick={() => (ui.searchOpen = true)}><Search size={19} /></button>
    {/if}
    <button class="icon-btn" aria-label={t('Configurações')} title="{t('Configurações')} ({mod},)" onclick={() => (ui.settingsOpen = true)}><Settings size={19} /></button>
  </div>
</header>

<MapThemeSheet bind:open={ui.themeOpen} />

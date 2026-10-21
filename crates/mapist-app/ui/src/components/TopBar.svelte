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
    PanelRight,
    Plus,
    Redo2,
    Save,
    Search,
    Settings,
    Trash2,
    Undo2,
    X,
  } from '@lucide/svelte'
  import { doc } from '../lib/document.svelte'
  import { prefs } from '../lib/prefs.svelte'
  import { ui } from '../lib/ui.svelte'

  const mod = /Mac/.test(navigator.platform) ? '⌘' : 'Ctrl+'
  const sel = $derived(doc.selectedNode)
  const title = $derived(doc.snapshot.title || 'Sem título')

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

<header class="topbar appbar">
  <div class="top-left">
    <button class="icon-btn" aria-label="Novo mapa" title="Novo mapa ({mod}N)" onclick={() => doc.newMap()}><FilePlus2 size={20} /></button>
    <button class="icon-btn" aria-label="Abrir" title="Abrir ({mod}O)" onclick={() => doc.openDialog()}><FolderOpen size={20} /></button>
    <button class="icon-btn" aria-label="Salvar" title="Salvar ({mod}S)" onclick={() => doc.saveDialog()}><Save size={20} /></button>
    <span class="doc-title" title={doc.snapshot.path ?? title}>
      <span class="grow">{title}</span>
      {#if doc.snapshot.dirty}<i class="dirty" aria-label="Alterações não salvas" title="Alterações não salvas"></i>{/if}
    </span>
  </div>

  <div class="top-actions">
    {#if sel}
      <div class="filter-group" role="group" aria-label="Nó selecionado">
        <button class="icon-btn" aria-label="Adicionar filho" title="Adicionar filho (Tab)" onclick={() => doc.addChild()}><Plus size={20} /></button>
        {#if sel.has_children}
          <button class="icon-btn" aria-label={sel.folded ? 'Expandir ramo' : 'Recolher ramo'} title="{sel.folded ? 'Expandir ramo' : 'Recolher ramo'} (Espaço)" onclick={() => doc.toggleFold()}>
            {#if sel.folded}<ChevronsUpDown size={19} />{:else}<ChevronsDownUp size={19} />{/if}
          </button>
        {/if}
        {#if sel.parent !== null}
          <button class="icon-btn" aria-label="Excluir nó" title="Excluir nó (Delete)" onclick={() => doc.remove()}><Trash2 size={19} /></button>
        {/if}
      </div>
    {/if}
    {#if doc.snapshot.can_undo || doc.snapshot.can_redo}
      <div class="filter-group" role="group" aria-label="Histórico">
        <button class="icon-btn" aria-label="Desfazer" title="Desfazer ({mod}Z)" disabled={!doc.snapshot.can_undo} onclick={() => doc.undo()}><Undo2 size={19} /></button>
        <button class="icon-btn" aria-label="Refazer" title="Refazer ({mod}⇧Z)" disabled={!doc.snapshot.can_redo} onclick={() => doc.redo()}><Redo2 size={19} /></button>
      </div>
    {/if}
    {#if searchShown}
      <label class="search inline">
        <Search size={16} aria-hidden="true" />
        <input
          id="busca"
          bind:this={input}
          value={query}
          placeholder="Buscar no mapa"
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
          <button class="icon-btn sm" aria-label="Limpar busca" onclick={() => { setQuery(''); input?.focus() }}><X size={14} /></button>
        {/if}
      </label>
    {:else}
      <button class="icon-btn" aria-label="Buscar" title="Buscar ({mod}F)" onclick={() => (ui.searchOpen = true)}><Search size={19} /></button>
    {/if}
    <button
      class="icon-btn"
      class:on={prefs.inspectorOpen}
      aria-label={prefs.inspectorOpen ? 'Fechar painel' : 'Abrir painel'}
      aria-pressed={prefs.inspectorOpen}
      title={prefs.inspectorOpen ? 'Fechar painel' : 'Abrir painel'}
      onclick={() => prefs.toggleInspector()}
    ><PanelRight size={19} /></button>
    <button class="icon-btn" aria-label="Configurações" title="Configurações ({mod},)" onclick={() => (ui.settingsOpen = true)}><Settings size={19} /></button>
  </div>
</header>

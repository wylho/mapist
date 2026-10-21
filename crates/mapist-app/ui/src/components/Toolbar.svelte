<script lang="ts">
  // Barra superior, no desenho da appbar do Ideario: ações de arquivo e de nó à
  // esquerda; à direita, busca, desfazer/refazer, aparência e painel.
  // Ícones do @lucide/svelte, como no Ideario: o tamanho vem do componente.
  import {
    FilePlus2,
    FolderOpen,
    Save,
    Plus,
    Trash2,
    ChevronsDownUp,
    ChevronsUpDown,
    Search,
    X,
    Undo2,
    Redo2,
    Sun,
    Moon,
    PanelRight,
  } from '@lucide/svelte'
  import { doc } from '../lib/document.svelte'
  import { theme } from '../lib/theme.svelte'
  import { prefs } from '../lib/prefs.svelte'

  const mod = typeof navigator !== 'undefined' && /Mac/.test(navigator.platform) ? '⌘' : 'Ctrl+'

  let search = $state(prefs.search)
  let searchTimer: ReturnType<typeof setTimeout> | undefined
  let input: HTMLInputElement | undefined = $state()

  function setSearch(q: string) {
    search = q
    prefs.setSearch(q)
    clearTimeout(searchTimer)
    searchTimer = setTimeout(() => doc.search(q), 160)
  }

  const sel = $derived(doc.selectedNode)
</script>

<header class="topbar appbar mapbar">
  <div class="top-left">
    <button class="icon-btn" aria-label="Novo mapa" title="Novo mapa ({mod}N)" onclick={() => doc.newMap()}>
      <FilePlus2 size={19} />
    </button>
    <button class="icon-btn" aria-label="Abrir" title="Abrir ({mod}O)" onclick={() => doc.openDialog()}>
      <FolderOpen size={19} />
    </button>
    <button class="icon-btn" aria-label="Salvar" title="Salvar ({mod}S)" onclick={() => doc.saveDialog()}>
      <Save size={19} />
    </button>

    <span class="bar-sep" aria-hidden="true"></span>

    <button class="icon-btn" aria-label="Adicionar filho" title="Adicionar filho (Tab)" disabled={!sel} onclick={() => doc.addChild()}>
      <Plus size={20} />
    </button>
    <button class="icon-btn" aria-label="Excluir nó" title="Excluir nó (Delete)" disabled={!sel || sel.parent === null} onclick={() => doc.remove()}>
      <Trash2 size={19} />
    </button>
    <button
      class="icon-btn"
      aria-label={sel?.folded ? 'Expandir ramo' : 'Recolher ramo'}
      title="{sel?.folded ? 'Expandir ramo' : 'Recolher ramo'} (Espaço)"
      disabled={!sel?.has_children}
      onclick={() => doc.toggleFold()}
    >
      {#if sel?.folded}<ChevronsUpDown size={19} />{:else}<ChevronsDownUp size={19} />{/if}
    </button>
  </div>

  <div class="top-actions">
    <label class="search inline">
      <Search size={16} aria-hidden="true" />
      <input
        bind:this={input}
        type="text"
        placeholder="Buscar nós"
        value={search}
        autocomplete="off"
        spellcheck="false"
        oninput={(e) => setSearch(e.currentTarget.value)}
        onkeydown={(e) => {
          if (e.key === 'Escape') {
            if (search) setSearch('')
            else e.currentTarget.blur()
          }
        }}
      />
      {#if search}
        <button class="icon-btn sm" aria-label="Limpar busca" onclick={() => { setSearch(''); input?.focus() }}><X size={14} /></button>
      {/if}
    </label>
    <button class="icon-btn" aria-label="Desfazer" title="Desfazer ({mod}Z)" disabled={!doc.snapshot.can_undo} onclick={() => doc.undo()}>
      <Undo2 size={19} />
    </button>
    <button class="icon-btn" aria-label="Refazer" title="Refazer ({mod}⇧Z)" disabled={!doc.snapshot.can_redo} onclick={() => doc.redo()}>
      <Redo2 size={19} />
    </button>
    <button
      class="icon-btn"
      aria-label={theme.mode === 'dark' ? 'Modo claro' : 'Modo escuro'}
      title={theme.mode === 'dark' ? 'Modo claro' : 'Modo escuro'}
      onclick={() => theme.toggleMode()}
    >
      {#if theme.mode === 'dark'}<Sun size={19} />{:else}<Moon size={19} />{/if}
    </button>
    <button
      class="icon-btn"
      class:on={prefs.inspectorOpen}
      aria-label={prefs.inspectorOpen ? 'Fechar painel' : 'Abrir painel'}
      aria-pressed={prefs.inspectorOpen}
      title={prefs.inspectorOpen ? 'Fechar painel' : 'Abrir painel'}
      onclick={() => prefs.toggleInspector()}
    >
      <PanelRight size={19} />
    </button>
  </div>
</header>

<script lang="ts">
  // Abas dos mapas abertos, no canto de cima do mapa (pedido do usuário: dentro da área do mapa, sem mexer na barra
  // de cima). Mesmo desenho flutuante da barra de zoom. Clique troca; × ou o botão do meio fecha (pergunta se há algo
  // por salvar); + abre um mapa novo.
  import { Plus, X } from '@lucide/svelte'
  import { doc } from '../lib/document.svelte'

  const mod = typeof navigator !== 'undefined' && /Mac|iPhone|iPad/.test(navigator.platform) ? '⌘' : 'Ctrl+'
</script>

{#if doc.tabs.length}
  <div class="map-tabs" role="tablist" tabindex="-1" aria-label="Mapas abertos" onpointerdown={(e) => e.stopPropagation()} ondblclick={(e) => e.stopPropagation()}>
    {#each doc.tabs as t (t.id)}
      {@const on = t.id === doc.snapshot.active}
      {@const name = doc.tabName(t)}
      <div class="map-tab" class:on title={t.path ?? name}>
        <button
          class="map-tab-name"
          role="tab"
          aria-selected={on}
          onclick={() => void doc.switchTab(t.id)}
          onauxclick={(e) => e.button === 1 && void doc.closeTab(t.id)}
        >
          {#if t.dirty}<i class="map-tab-dirty" aria-label="não salvo"></i>{/if}
          <span>{name}</span>
        </button>
        <button class="map-tab-close" aria-label="Fechar {name}" title="Fechar ({mod}W)" onclick={() => void doc.closeTab(t.id)}><X size={13} /></button>
      </div>
    {/each}
    <button class="map-tab-new" aria-label="Novo mapa" title="Novo mapa ({mod}N)" onclick={() => void doc.newMap()}><Plus size={16} /></button>
  </div>
{/if}

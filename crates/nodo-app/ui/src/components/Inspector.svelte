<script lang="ts">
  // Inspetor do tópico selecionado: o que é do mapa (lado do ramo, filhos, nível). O conteúdo do tópico (título, texto,
  // emoji, etiquetas, lembrete, status, cor) se edita no próprio card, como no Ideario.
  import { doc } from '../lib/document.svelte'
  import { notes } from '../lib/notes.svelte'
  import { SquareArrowOutUpRight } from '@lucide/svelte'

  const node = $derived(doc.selectedNode)
</script>

<aside class="inspector">
  <div class="inspector-head">
    <span class="inspector-title">Inspetor</span>
  </div>

  <div class="inspector-body">
    {#if !node}
      <div class="banner">Selecione um nó para editá-lo.</div>
    {:else}
      <button class="btn full" onclick={() => notes.show(node.id)}><SquareArrowOutUpRight size={15} />Abrir card</button>

      {#if node.depth === 1}
        <div class="card">
          <div class="card-body field">
            <span class="field-label">Lado do ramo</span>
            <div class="seg tight" role="radiogroup" aria-label="Lado do ramo">
              {#each [{ v: 'left', l: 'Esquerda' }, { v: 'right', l: 'Direita' }] as o (o.v)}
                <button
                  class="seg-item"
                  role="radio"
                  aria-checked={node.side === o.v}
                  data-state={node.side === o.v ? 'on' : 'off'}
                  onclick={() => node.side !== o.v && doc.setSide(node.id, o.v as 'left' | 'right')}>{o.l}</button
                >
              {/each}
            </div>
          </div>
        </div>
      {/if}

      <div class="card">
        <div class="card-body field">
          <span class="field-label">Nó</span>
          <div class="stats">
            <div><b>{node.child_ids.length}</b><span>filhos</span></div>
            <div><b>{node.depth}</b><span>nível</span></div>
          </div>
        </div>
      </div>
    {/if}


    <div class="banner">
      {doc.snapshot.nodes.length === 1 ? '1 nó' : `${doc.snapshot.nodes.length} nós`}
    </div>
  </div>
</aside>

<script lang="ts">
  // Inspetor do nó selecionado: texto, nota, cor, lado, layout e métricas.
  import { doc } from '../lib/document.svelte'
  import { theme } from '../lib/theme.svelte'
  import { NODE_COLOR_TOKENS, resolveNodeColor } from '../lib/palettes'
  import { notes } from '../lib/notes.svelte'
  import { StickyNote } from '@lucide/svelte'

  let text = $state('')
  let editingId = $state<number | null>(null)
  let textTimer: ReturnType<typeof setTimeout> | undefined

  const node = $derived(doc.selectedNode)
  const COLOR_NAMES: Record<string, string> = {
    '--n-sand': 'Areia',
    '--n-sage': 'Sálvia',
    '--n-sky': 'Céu',
    '--n-rose': 'Rosa',
    '--n-lilac': 'Lilás',
    '--n-butter': 'Manteiga',
  }

  // Sincroniza os campos quando a seleção muda, sem atropelar quem está digitando.
  $effect(() => {
    const n = doc.selectedNode
    if (!n) {
      editingId = null
      text = ''
      return
    }
    if (n.id !== editingId) {
      editingId = n.id
      text = n.text
      return
    }
    // O mesmo nó mudou por fora (edição no mapa, desfazer, o Claude): acompanha, menos o campo onde se está digitando.
    const focused = document.activeElement?.id
    if (focused !== 'node-text') text = n.text
  })

  function onText(e: Event) {
    text = (e.target as HTMLTextAreaElement).value
    const n = doc.selectedNode
    if (!n) return
    clearTimeout(textTimer)
    textTimer = setTimeout(() => doc.setText(n.id, text || ' '), 320)
  }

  function swatchColor(token: string): string {
    const s = getComputedStyle(document.documentElement)
    return s.getPropertyValue(token).trim() || 'transparent'
  }
</script>

<aside class="inspector">
  <div class="inspector-head">
    <span class="inspector-title">Inspetor</span>
  </div>

  <div class="inspector-body">
    {#if !node}
      <div class="banner">Selecione um nó para editá-lo.</div>
    {:else}
      <div class="card">
        <div class="card-body field">
          <label class="field-label" for="node-text">Texto</label>
          <textarea id="node-text" rows="2" placeholder="Texto do nó" value={text} oninput={onText}
          ></textarea>
        </div>
      </div>

      <div class="card">
        <div class="card-body field">
          <!-- A nota se escreve no próprio card, ao lado do nó; aqui só a prévia e o atalho para abrir. -->
          <span class="field-label">Nota</span>
          {#if node.note.trim()}
            <p class="note-peek">{node.note}</p>
          {/if}
          <button class="btn" onclick={() => notes.show(node.id, !node.note.trim() || notes.isOpen(node.id))}>
            <StickyNote size={15} />{node.note.trim() ? (notes.isOpen(node.id) ? 'Escrever na nota' : 'Abrir no mapa') : 'Adicionar nota'}
          </button>
        </div>
      </div>

      <div class="card">
        <div class="card-body field">
          <span class="field-label">Cor do ramo</span>
          <div class="swatches">
            <button
              class="swatch none"
              title="Sem cor"
              aria-pressed={node.color === null}
              onclick={() => doc.setColor(node.id, null)}>×</button
            >
            {#each NODE_COLOR_TOKENS as token (token)}
              <button
                class="swatch"
                style:background={swatchColor(token)}
                title={COLOR_NAMES[token] ?? token}
                aria-label={COLOR_NAMES[token] ?? token}
                aria-pressed={node.color === resolveNodeColor(token, theme.colors)}
                onclick={() => doc.setColor(node.id, resolveNodeColor(token, theme.colors))}
              ></button>
            {/each}
          </div>
        </div>
      </div>

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

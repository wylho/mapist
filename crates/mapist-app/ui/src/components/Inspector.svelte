<script lang="ts">
  // Inspetor do nó selecionado: texto, nota, cor, lado, layout e métricas.
  import { doc } from '../lib/document.svelte'
  import { theme } from '../lib/theme.svelte'
  import { NODE_COLOR_TOKENS, resolveNodeColor } from '../lib/palettes'

  let text = $state('')
  let note = $state('')
  let editingId = $state<number | null>(null)
  let textTimer: ReturnType<typeof setTimeout> | undefined
  let noteTimer: ReturnType<typeof setTimeout> | undefined

  const node = $derived(doc.selectedNode)
  const mod = /Mac/.test(navigator.platform) ? '⌘' : 'Ctrl+'
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
      note = ''
      return
    }
    if (n.id !== editingId) {
      editingId = n.id
      text = n.text
      note = n.note
    }
  })

  function onText(e: Event) {
    text = (e.target as HTMLTextAreaElement).value
    const n = doc.selectedNode
    if (!n) return
    clearTimeout(textTimer)
    textTimer = setTimeout(() => doc.setText(n.id, text || ' '), 320)
  }

  function onNote(e: Event) {
    note = (e.target as HTMLTextAreaElement).value
    const n = doc.selectedNode
    if (!n) return
    clearTimeout(noteTimer)
    noteTimer = setTimeout(() => doc.setNote(n.id, note), 420)
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
          <label class="field-label" for="node-note">
            Nota <span class="field-hint">markdown</span>
          </label>
          <textarea id="node-note" rows="4" placeholder="Adicione uma nota…" value={note} oninput={onNote}
          ></textarea>
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

    <div class="field">
      <span class="field-label">Atalhos</span>
      <div class="keys">
        <div class="keys-row"><span>Adicionar filho</span><kbd>Tab</kbd></div>
        <div class="keys-row"><span>Adicionar irmão</span><kbd>↵</kbd></div>
        <div class="keys-row"><span>Excluir</span><kbd>Delete</kbd></div>
        <div class="keys-row"><span>Recolher ou expandir</span><kbd>Espaço</kbd></div>
        <div class="keys-row"><span>Desfazer</span><kbd>{mod}Z</kbd></div>
        <div class="keys-row"><span>Buscar</span><kbd>{mod}F</kbd></div>
      </div>
    </div>

    <div class="banner">
      {doc.snapshot.nodes.length === 1 ? '1 nó' : `${doc.snapshot.nodes.length} nós`}
    </div>
  </div>
</aside>

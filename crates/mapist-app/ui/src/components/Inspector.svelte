<script lang="ts">
  // Inspetor do nó selecionado: texto, nota, cor, lado, layout e métricas.
  import { doc } from '../lib/document.svelte'
  import { theme } from '../lib/theme.svelte'
  import McpPanel from './McpPanel.svelte'

  const NODE_TOKENS = ['--n-sand', '--n-sage', '--n-sky', '--n-rose', '--n-lilac', '--n-butter'] as const

  let text = $state('')
  let note = $state('')
  let editingId = $state<number | null>(null)
  let textTimer: ReturnType<typeof setTimeout> | undefined
  let noteTimer: ReturnType<typeof setTimeout> | undefined

  const node = $derived(doc.selectedNode)

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
    {#if node}
      <span class="pill tag">#{node.id}</span>
    {/if}
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
            {#each NODE_TOKENS as token (token)}
              <button
                class="swatch"
                style:background={swatchColor(token)}
                title={token}
                aria-pressed={node.color === token}
                onclick={() => doc.setColor(node.id, token)}
              ></button>
            {/each}
          </div>
        </div>
      </div>

      <div class="card">
        <div class="card-body field">
          <span class="field-label">Nó</span>
          <div class="stats">
            <div><b>{node.child_ids.length}</b><span>filhos</span></div>
            <div><b>{node.depth}</b><span>nível</span></div>
            <div><b>{Math.round(node.width)}×{Math.round(node.height)}</b><span>tamanho</span></div>
          </div>
        </div>
      </div>
    {/if}

    <div class="field">
      <span class="field-label">Atalhos</span>
      <div class="keys">
        <div class="keys-row"><span>Adicionar filho</span><kbd>Tab</kbd></div>
        <div class="keys-row"><span>Adicionar irmão</span><kbd>↵</kbd></div>
        <div class="keys-row"><span>Excluir</span><kbd>⌫</kbd></div>
        <div class="keys-row"><span>Dobrar</span><kbd>Space</kbd></div>
        <div class="keys-row"><span>Desfazer</span><kbd>⌘Z</kbd></div>
      </div>
    </div>

    <McpPanel />

    <div class="banner">
      {doc.snapshot.nodes.length === 1 ? '1 nó' : `${doc.snapshot.nodes.length} nós`} · tema {theme.label}
    </div>
  </div>
</aside>

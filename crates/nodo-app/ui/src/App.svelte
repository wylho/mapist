<script lang="ts">
  // Raiz da aplicação, com a casca do Ideario: barra superior, corpo (canvas + inspetor), aviso, confirmação e
  // configurações. Os atalhos globais ficam aqui.
  import { onMount } from 'svelte'
  import { Maximize, Minus, Plus } from '@lucide/svelte'
  import TopBar from './components/TopBar.svelte'
  import Canvas from './components/Canvas.svelte'
  import Inspector from './components/Inspector.svelte'
  import SettingsSheet from './components/SettingsSheet.svelte'
  import ConfirmDialog from './components/ConfirmDialog.svelte'
  import UpdateNotice from './components/UpdateNotice.svelte'
  import { doc } from './lib/document.svelte'
  import { hasHost, isPreview } from './lib/api'
  import { prefs } from './lib/prefs.svelte'
  import { ui } from './lib/ui.svelte'

  let canvasRef = $state<Canvas | undefined>(undefined)
  // O nível de zoom vive no canvas; espelhamos aqui para o rótulo reagir.
  let zoom = $state(prefs.zoom)
  const zoomPct = $derived(Math.round(zoom * 100))

  const typing = (e: KeyboardEvent) => !!(e.target as Element | null)?.closest?.('input, textarea, [contenteditable="true"]')

  function onKeydown(e: KeyboardEvent) {
    // Com uma folha ou confirmação aberta, as teclas são dela.
    if (ui.settingsOpen || ui.confirmRequest) return
    const mod = e.metaKey || e.ctrlKey
    const key = e.key.toLowerCase()

    if (mod && !e.altKey) {
      const action: Record<string, () => void> = {
        z: () => (e.shiftKey ? doc.redo() : doc.undo()),
        y: () => doc.redo(),
        s: () => doc.saveDialog(e.shiftKey),
        o: () => doc.openDialog(),
        n: () => doc.newMap(),
        f: () => {
          ui.searchOpen = true
          document.getElementById('busca')?.focus()
        },
        ',': () => (ui.settingsOpen = true),
        '0': () => canvasRef?.fit(true),
        // Recolher/expandir, como no XMind (o Espaço agora edita).
        '/': () => doc.toggleFold(),
      }
      // Desfazer dentro de um campo de texto é do campo.
      if (key in action && !(typing(e) && (key === 'z' || key === 'y'))) {
        e.preventDefault()
        action[key]()
      }
      return
    }
    if (typing(e) || doc.editing) return

    switch (e.key) {
      case 'Tab':
        e.preventDefault()
        void doc.addChild()
        return
      case 'Enter':
        e.preventDefault()
        void doc.addSibling()
        return
      case ' ':
      case 'F2':
        e.preventDefault()
        doc.startEdit()
        return
      case 'Backspace':
      case 'Delete':
        e.preventDefault()
        void doc.remove()
        return
      case 'ArrowUp':
        e.preventDefault()
        doc.moveSelection(0, -1)
        return
      case 'ArrowDown':
        e.preventDefault()
        doc.moveSelection(0, 1)
        return
      case 'ArrowLeft':
        e.preventDefault()
        doc.moveSelection(-1, 0)
        return
      case 'ArrowRight':
        e.preventDefault()
        doc.moveSelection(1, 0)
        return
    }
    // Qualquer letra, número ou símbolo com um nó selecionado começa a escrever nele, substituindo o texto (XMind).
    if (e.key.length === 1 && !e.altKey && doc.selectedNode) {
      e.preventDefault()
      doc.startEdit(e.key)
    }
  }

  onMount(() => {
    doc.load()
    if (!hasHost) return
    // Fechar (ou Cmd+Q) com alterações não salvas: o núcleo segura e pergunta aqui.
    const unlisten: (() => void)[] = []
    void import('@tauri-apps/api/event').then(async ({ listen }) => {
      unlisten.push(await listen('close-requested', () => void doc.confirmQuit()))
      // macOS: um .minder aberto pelo Finder com o app já aberto.
      unlisten.push(await listen<string>('open-file', (e) => void doc.openPath(e.payload)))
    })
    return () => unlisten.forEach((u) => u())
  })
</script>

<!-- Ao voltar para a janela, confere se um agente salvou o arquivo pelo MCP. -->
<svelte:window onkeydown={onKeydown} onfocus={() => void doc.checkDisk()} />

<div class="shell map-shell">
  <TopBar />

  <div class="map-body">
    <Canvas bind:this={canvasRef} onzoom={(z) => { zoom = z; prefs.setZoom(z) }}>
      <div class="zoombar">
        <button aria-label="Diminuir zoom" title="Diminuir" onclick={() => canvasRef?.zoomBy(1 / 1.2)}><Minus size={17} /></button>
        <span class="zoomlevel">{zoomPct}%</span>
        <button aria-label="Aumentar zoom" title="Aumentar" onclick={() => canvasRef?.zoomBy(1.2)}><Plus size={17} /></button>
        <button aria-label="Ajustar à tela" title="Ajustar à tela" onclick={() => canvasRef?.fit(true)}><Maximize size={17} /></button>
      </div>
    </Canvas>

    {#if prefs.inspectorOpen}
      <Inspector />
    {/if}
  </div>

  <!-- O que dá para fazer agora: tira a dúvida de "já posso digitar?". Só com teclado, então não no celular. -->
  {#if doc.hasDocument}
    <div class="key-hint" aria-live="polite">
      {#if doc.editing}
        <span><kbd>Enter</kbd> confirma</span><span><kbd>Tab</kbd> confirma e cria filho</span><span><kbd>Esc</kbd> cancela</span>
      {:else if doc.selectedNode}
        <span>Digite para escrever</span><span><kbd>Tab</kbd> filho</span><span><kbd>Enter</kbd> irmão</span><span><kbd>Espaço</kbd> editar</span>
      {/if}
    </div>
  {/if}

  {#if isPreview}
    <div class="preview-tag" title="Backend simulado em memória; nenhum arquivo é lido ou gravado">
      <span class="dot"></span> Prévia: dados de exemplo
    </div>
  {:else if !hasHost}
    <div class="preview-tag">Rodando fora do app: sem acesso ao documento</div>
  {/if}

  <div class="toast" class:show={!!ui.toast} role="status" aria-live="polite">
    <span>{ui.toast}</span>
  </div>

  <UpdateNotice />
  <SettingsSheet />
  <ConfirmDialog />
</div>

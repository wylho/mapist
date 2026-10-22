<script lang="ts">
  // Raiz da aplicação, com a casca do Ideario: barra superior, corpo (canvas + inspetor), aviso, confirmação e
  // configurações. Os atalhos globais ficam aqui.
  import { onMount } from 'svelte'
  import { Maximize, Minus, Plus } from '@lucide/svelte'
  import TopBar from './components/TopBar.svelte'
  import Canvas from './components/Canvas.svelte'
  import TabBar from './components/TabBar.svelte'
  import { notes } from './lib/notes.svelte'
  import SettingsSheet from './components/SettingsSheet.svelte'
  import ConfirmDialog from './components/ConfirmDialog.svelte'
  import UpdateNotice from './components/UpdateNotice.svelte'
  import { MenuAt, t, tn } from '@wylho/atrio'
  import ExportSheet from './components/ExportSheet.svelte'
  import { backgroundMenu, groupMenu, nodeMenu, type ViewActions } from './lib/mapmenus'
  import { cardsToClipboard, readCards } from '@wylho/atrio'
  import { doc } from './lib/document.svelte'
  import { hasHost, isPreview } from './lib/api'
  import { prefs } from './lib/prefs.svelte'
  import { ui } from './lib/ui.svelte'

  let canvasRef = $state<Canvas | undefined>(undefined)
  let menuRef = $state<MenuAt | undefined>(undefined)
  const viewActions: ViewActions = {
    fit: () => canvasRef?.fit(true),
    fitSelection: () => canvasRef?.fitSelection(),
    actualSize: () => canvasRef?.actualSize(),
    addNoteAt: (x, y) => void canvasRef?.addNoteAt(x, y),
  }

  /** Menu de contexto do mapa: do nó, se houver um, senão do fundo. */
  function openMenu(x: number, y: number, id: number | null) {
    const items = id === null ? backgroundMenu(viewActions, { x, y }) : doc.grouped && doc.isPicked(id) ? groupMenu() : nodeMenu(id, viewActions)
    menuRef?.openAt(x, y, items)
  }
  // O nível de zoom vive no canvas; espelhamos aqui para o rótulo reagir.
  let zoom = $state(prefs.zoom)
  const zoomPct = $derived(Math.round(zoom * 100))
  const modKey = /Mac/.test(navigator.platform) ? '⌘' : 'Ctrl+'

  const typing = (e: Event) => !!(e.target as Element | null)?.closest?.('input, textarea, [contenteditable="true"]')

  // Copiar e colar um nó como card do Atrio: o Ideario cola como nota, e uma nota copiada no Ideario cola aqui como nó.
  // Num campo de texto, copiar e colar são do campo.
  function onCopy(e: ClipboardEvent) {
    if (typing(e) || !e.clipboardData) return
    // Um grupo escolhido vai inteiro, um card por tópico.
    const cards = doc.grouped ? doc.groupCards() : [doc.selectedCard()].filter((c) => c !== null)
    if (!cards.length) return
    e.preventDefault()
    const { html, text } = cardsToClipboard(cards)
    e.clipboardData.setData('text/html', html)
    e.clipboardData.setData('text/plain', text)
    ui.say(cards.length > 1 ? tn(cards.length, 'Copiado como card', 'Copiados {n} cards') : t('Copiado como card'))
  }

  function onPaste(e: ClipboardEvent) {
    if (typing(e)) return
    const cards = readCards(e.clipboardData)
    if (!cards.length) return
    e.preventDefault()
    void doc.pasteCards(cards)
  }

  function onKeydown(e: KeyboardEvent) {
    // Com uma folha ou confirmação aberta, as teclas são dela.
    if (ui.settingsOpen || ui.confirmRequest) return
    const mod = e.metaKey || e.ctrlKey
    const key = e.key.toLowerCase()

    // Ctrl+Tab e Ctrl+⇧Tab passam de aba, em todos os sistemas (como nos navegadores).
    if (e.ctrlKey && e.key === 'Tab') {
      e.preventDefault()
      void doc.cycleTab(e.shiftKey ? -1 : 1)
      return
    }

    if (mod && !e.altKey) {
      const action: Record<string, () => void> = {
        z: () => (e.shiftKey ? doc.redo() : doc.undo()),
        y: () => doc.redo(),
        // ⌘A escolhe todos os tópicos e notas soltas.
        a: () => doc.pickAll(),
        s: () => doc.saveDialog(e.shiftKey),
        // ⌘⇧E exporta (⌘E sozinho fica livre, como no XMind).
        e: () => e.shiftKey && (ui.exportOpen = true),
        o: () => doc.openDialog(),
        n: () => doc.newMap(),
        t: () => doc.newMap(),
        // ⌘Enter abre o card do tópico, para escrever com detalhes sem tirar a mão do teclado.
        enter: () => doc.selected !== null && notes.show(doc.selected, true),
        w: () => doc.closeTab(),
        f: () => {
          ui.searchOpen = true
          document.getElementById('busca')?.focus()
        },
        ',': () => (ui.settingsOpen = true),
        // Zoom como no XMind e no MindNode: ⌘= aproxima, ⌘- afasta, ⌘0 volta ao tamanho real.
        '=': () => canvasRef?.zoomBy(1.2),
        '+': () => canvasRef?.zoomBy(1.2),
        '-': () => canvasRef?.zoomBy(1 / 1.2),
        '0': () => canvasRef?.actualSize(),
        // Recolher/expandir, como no XMind (o Espaço agora edita).
        '/': () => doc.toggleFold(),
      }
      // Desfazer dentro de um campo de texto é do campo.
      if (key in action && !(typing(e) && (key === 'z' || key === 'y' || key === 'enter' || key === 'a'))) {
        e.preventDefault()
        action[key]()
      }
      return
    }
    if (typing(e)) return
    // Tecla de menu ou ⇧F10: o menu do nó selecionado, como o clique direito.
    if ((e.key === 'ContextMenu' || (e.shiftKey && e.key === 'F10')) && !doc.editing) {
      const at = canvasRef?.selectionAnchor()
      if (at && doc.selected !== null) {
        e.preventDefault()
        openMenu(at.x, at.y, doc.selected)
      }
      return
    }
    // Enquadrar, como no Figma e no Miro: ⇧1 o mapa inteiro, ⇧2 a seleção. Pelo código da tecla, porque ⇧1 é "!"
    // num teclado e outra coisa em outro.
    if (e.shiftKey && !e.altKey && !doc.editing && (e.code === 'Digit1' || e.code === 'Digit2')) {
      e.preventDefault()
      if (e.code === 'Digit1') canvasRef?.fit(true)
      else canvasRef?.fitSelection()
      return
    }
    // Um nó nascendo ou o campo abrindo: letras vão para ele (doc.type), o resto espera.
    if (doc.creating || doc.editing) {
      if (e.key.length === 1 && !e.altKey) {
        e.preventDefault()
        doc.type(e.key)
      } else if (doc.creating) {
        e.preventDefault()
      }
      return
    }

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
        // O Espaço edita ao soltar: segurado, ele vira a mão que move o mapa (Espaço + arrastar).
        e.preventDefault()
        if (!e.repeat) {
          ui.spaceHeld = true
          ui.spacePanned = false
        }
        return
      case 'F2':
        e.preventDefault()
        doc.startEdit()
        return
      case 'Escape':
        if (doc.grouped) {
          e.preventDefault()
          doc.clearGroup()
        }
        return
      case 'Backspace':
      case 'Delete':
        e.preventDefault()
        void (doc.grouped ? doc.removeGroup() : doc.remove())
        return
      case 'ArrowUp':
        e.preventDefault()
        // ⌥↑ sobe o tópico entre os irmãos, como no XMind; sem ⌥, anda pelo mapa.
        if (e.altKey) void doc.reorder(-1)
        else doc.moveSelection(0, -1)
        return
      case 'ArrowDown':
        e.preventDefault()
        if (e.altKey) void doc.reorder(1)
        else doc.moveSelection(0, 1)
        return
      case 'Home':
        e.preventDefault()
        doc.selectRoot()
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
      doc.type(e.key)
    }
  }

  function onKeyup(e: KeyboardEvent) {
    if (e.key !== ' ' || !ui.spaceHeld) return
    ui.spaceHeld = false
    // Soltou sem ter arrastado: é o Espaço de editar o tópico.
    if (!ui.spacePanned && !typing(e) && !doc.editing && !doc.creating && !ui.settingsOpen && !ui.confirmRequest) doc.startEdit()
    ui.spacePanned = false
  }

  onMount(() => {
    doc.load()
    if (!hasHost) return
    // Fechar (ou Cmd+Q) com alterações não salvas: o núcleo segura e pergunta aqui.
    const unlisten: (() => void)[] = []
    void import('@tauri-apps/api/event').then(async ({ listen }) => {
      unlisten.push(await listen('close-requested', () => void doc.confirmQuit()))
      // macOS: um mapa aberto pelo Finder com o app já aberto.
      unlisten.push(await listen<string>('open-file', (e) => void doc.openPath(e.payload)))
    })
    return () => unlisten.forEach((u) => u())
  })
</script>

<!-- Ao voltar para a janela, confere se um agente salvou o arquivo pelo MCP. -->
<svelte:window onkeydown={onKeydown} onkeyup={onKeyup} onblur={() => (ui.spaceHeld = false)} oncopy={onCopy} onpaste={onPaste} onfocus={() => void doc.checkDisk()} />

<div class="shell map-shell">
  <TopBar />

  <div class="map-body">
    <Canvas bind:this={canvasRef} oncontext={openMenu} onzoom={(z) => { zoom = z; prefs.setZoom(z) }}>
      <TabBar />
      <div class="zoombar">
        <button aria-label={t('Diminuir zoom')} title={t('Diminuir ({tecla})', { tecla: `${modKey}-` })} onclick={() => canvasRef?.zoomBy(1 / 1.2)}><Minus size={17} /></button>
        <button class="zoomlevel" title={t('Tamanho real ({tecla})', { tecla: `${modKey}0` })} aria-label={t('Zoom em {pct}%; voltar a 100%', { pct: zoomPct })} onclick={() => canvasRef?.actualSize()}>{zoomPct}%</button>
        <button aria-label={t('Aumentar zoom')} title={t('Aumentar ({tecla})', { tecla: `${modKey}=` })} onclick={() => canvasRef?.zoomBy(1.2)}><Plus size={17} /></button>
        <button aria-label={t('Ver o mapa inteiro')} title="{t('Ver o mapa inteiro')} (⇧1)" onclick={() => canvasRef?.fit(true)}><Maximize size={17} /></button>
      </div>
    </Canvas>

  </div>

  <!-- O que dá para fazer agora: tira a dúvida de "já posso digitar?". Só com teclado, então não no celular. -->
  {#if doc.hasDocument}
    <div class="key-hint" aria-live="polite">
      {#if doc.editing}
        <span><kbd>Enter</kbd> {t('confirma')}</span><span><kbd>Tab</kbd> {t('confirma e cria filho')}</span><span><kbd>Esc</kbd> {t('cancela')}</span>
      {:else if doc.selectedNode}
        <span>{t('Digite para escrever')}</span><span><kbd>Tab</kbd> {t('filho')}</span><span><kbd>Enter</kbd> {t('irmão')}</span><span><kbd>{t('Espaço')}</kbd> {t('editar')}</span>
      {/if}
    </div>
  {/if}

  {#if isPreview}
    <div class="preview-tag" title={t('Backend simulado em memória; nenhum arquivo é lido ou gravado')}>
      <span class="dot"></span> {t('Prévia: dados de exemplo')}
    </div>
  {:else if !hasHost}
    <div class="preview-tag">{t('Rodando fora do app: sem acesso ao documento')}</div>
  {/if}

  <div class="toast" class:show={!!ui.toast} role="status" aria-live="polite">
    <span>{ui.toast}</span>
  </div>

  <UpdateNotice />
  <MenuAt bind:this={menuRef} />
  <ExportSheet />
  <SettingsSheet />
  <ConfirmDialog />
</div>

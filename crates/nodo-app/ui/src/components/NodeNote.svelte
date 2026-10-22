<script lang="ts">
  // Um nó aberto: o card do Atrio, o mesmo do Ideario, na mesma janela (`CardDialog`: diálogo na janela larga, tela
  // cheia na estreita, modo documento). Título = texto do nó; o texto vira o Markdown da nota no núcleo; emoji,
  // etiquetas, lembrete e status vão por `set_card`; a cor do card é a cor do ramo. Salva sozinho. Fotos, arquivos e
  // gravações vão no `.nodo`; categoria e prévia de link ficam desligadas.
  import { onDestroy, untrack } from 'svelte'
  import { DropdownMenu } from 'bits-ui'
  import { ArrowLeft, BookOpen, Check, ChevronRight, CircleDashed, Minimize2, MoreVertical, SmilePlus, Trash2 } from '@lucide/svelte'
  import {
    CardDialog,
    emptyMeta,
    features,
    hasStatus,
    ReminderPopover,
    STATUS,
    statusOf,
    type CardHost,
    type CardMeta,
    type TagCount,
  } from '@wylho/atrio'
  import { NoteEditor } from '@wylho/atrio/editor'
  import { api, attachments, hasHost, isPreview, type CardFields, type FlatNode } from '../lib/api'
  import { doc, nodeColorOf, noteColorOf } from '../lib/document.svelte'
  import { openLink } from '../lib/markdown'
  import { notes } from '../lib/notes.svelte'

  let { node, autofocus = false, onclose }: { node: FlatNode; autofocus?: boolean; onclose: () => void } = $props()

  // Fotos, arquivos, câmera e gravador entram no `.nodo` (um zip com os anexos). Categoria é só do Ideario; a prévia
  // de link fica desligada até o usuário decidir sobre buscar a página na internet.
  const FEATURES = features({ category: false, links: false })

  const id = untrack(() => node.id)

  // Modo documento só na janela larga, como no Ideario.
  const wideQuery = matchMedia('(min-width: 960px)')
  let wide = $state(wideQuery.matches)
  $effect(() => {
    const on = (e: MediaQueryListEvent) => (wide = e.matches)
    wideQuery.addEventListener('change', on)
    return () => wideQuery.removeEventListener('change', on)
  })

  function metaOf(n: FlatNode): CardMeta {
    const c = n.card ?? {}
    return emptyMeta({
      title: n.text,
      color: noteColorOf(n.color),
      emoji: c.emoji ?? null,
      tags: [...(c.tags ?? [])],
      reminderAt: c.reminder_at ?? null,
      reminderRepeat: (c.reminder_repeat ?? null) as CardMeta['reminderRepeat'],
      reminderDone: !!c.reminder_done,
      reminderDoing: !!c.reminder_doing,
      statusOn: !!c.status_on,
    })
  }

  const fieldsOf = (m: CardMeta): CardFields => ({
    tags: m.tags,
    emoji: m.emoji,
    reminder_at: m.reminderAt,
    reminder_repeat: m.reminderRepeat,
    reminder_done: m.reminderDone,
    reminder_doing: m.reminderDoing,
    status_on: m.statusOn,
  })

  // O que o núcleo tem agora, na forma de comparar: muda por aqui ou por fora (desfazer, um agente pelo MCP).
  const keyOf = (m: CardMeta) => JSON.stringify([m.title, m.color, fieldsOf(m)])

  const meta = $state<CardMeta>(untrack(() => metaOf(node)))
  let saved = untrack(() => keyOf(meta))
  let content = $state<unknown>(null)
  let lastNote = untrack(() => node.note)
  let card = $state<ReturnType<typeof NoteEditor> | undefined>(undefined)
  let bodyTimer: ReturnType<typeof setTimeout> | undefined
  let metaTimer: ReturnType<typeof setTimeout> | undefined
  let pendingBody: unknown = null

  void api.noteDoc(id).then((d) => (content = d))

  // Os campos mudaram no card: grava (um passo do desfazer por grupo de mudanças).
  $effect(() => {
    const key = keyOf(meta)
    if (key === saved) return
    clearTimeout(metaTimer)
    metaTimer = setTimeout(() => void saveMeta(), 350)
  })

  async function saveMeta() {
    clearTimeout(metaTimer)
    metaTimer = undefined
    const key = keyOf(meta)
    if (key === saved) return
    const n = doc.node(id)
    if (!n) return
    const was = metaOf(n)
    saved = key
    const title = meta.title.trim()
    if (title && title !== n.text) await doc.setText(id, title)
    if (meta.color !== was.color) await doc.setColor(id, nodeColorOf(meta.color))
    if (JSON.stringify(fieldsOf(meta)) !== JSON.stringify(fieldsOf(was))) await doc.setCard(id, fieldsOf(meta))
  }

  // O núcleo mudou por fora (desfazer, MCP): o card acompanha, se não houver nada nosso por gravar.
  $effect(() => {
    const n = doc.node(id)
    if (!n) return
    untrack(() => {
      const fresh = metaOf(n)
      if (!metaTimer && keyOf(fresh) !== saved && keyOf(fresh) !== keyOf(meta)) {
        Object.assign(meta, fresh)
        saved = keyOf(fresh)
      }
      if (!bodyTimer && n.note !== lastNote) {
        lastNote = n.note
        void api.noteDoc(id).then((d) => card?.getEditor()?.commands.setContent(d as never, { emitUpdate: false }))
      }
    })
  })

  function onbody(editor: { getJSON: () => unknown }) {
    pendingBody = editor.getJSON()
    clearTimeout(bodyTimer)
    bodyTimer = setTimeout(() => void saveBody(), 450)
  }

  async function saveBody() {
    clearTimeout(bodyTimer)
    const body = pendingBody
    pendingBody = null
    if (body == null) return (bodyTimer = undefined)
    await doc.setNoteDoc(id, body)
    lastNote = doc.node(id)?.note ?? lastNote
    bodyTimer = undefined
  }

  function flush() {
    if (metaTimer) void saveMeta()
    if (bodyTimer) void saveBody()
  }

  onDestroy(flush)

  // Arquivos soltos do computador sobre a janela, com o card aberto: entram nele (no app, o sistema entrega o caminho).
  $effect(() => {
    if (!hasHost || isPreview) return
    let stop: (() => void) | undefined
    let gone = false
    void import('@tauri-apps/api/webview').then(async ({ getCurrentWebview }) => {
      const off = await getCurrentWebview().onDragDropEvent((e) => {
        if (e.payload.type === 'drop' && e.payload.paths.length) card?.addFiles(e.payload.paths.map((path) => ({ path })))
      })
      if (gone) off()
      else stop = off
    })
    return () => {
      gone = true
      stop?.()
    }
  })

  function close() {
    flush()
    onclose()
  }

  async function removeTopic() {
    flush()
    onclose()
    doc.selected = id
    await doc.remove()
  }

  /** Etiquetas usadas no mapa, para sugerir. */
  function knownTags(): TagCount[] {
    const count = new Map<string, number>()
    for (const n of doc.snapshot.nodes) for (const t of n.card?.tags ?? []) count.set(t, (count.get(t) ?? 0) + 1)
    return [...count].map(([name, n]) => ({ name, count: n }))
  }

  const host: CardHost = {
    tags: knownTags,
    mediaUrl: (h) => attachments.url(h),
    imageUrl: (h) => attachments.url(h),
    importFile: (f, name) => attachments.importFile(f, name),
    importPath: (p) => attachments.importPath(p),
    attachments: (hs) => attachments.list(hs),
    download: (a) => void attachments.download(a),
    openAttachment: (a) => void attachments.open(a),
    openUrl: (url) => void openLink(url),
    markdownToDoc: (md) => api.markdownDoc(md) as Promise<{ content?: unknown[] }>,
  }
</script>

<CardDialog color={meta.color} doc={notes.docMode && wide} onclose={close}>
  {#if content}
    <NoteEditor
      bind:this={card}
      {meta}
      features={FEATURES}
      {host}
      {content}
      {onbody}
      {autofocus}
      dialogTitle
      placeholder="Escreva… use #tag para marcar"
    >
      {#snippet top()}
        <div class="ed-top">
          <button class="icon-btn" aria-label="Voltar ao mapa" onclick={close}><ArrowLeft size={20} /></button>
          <span class="ed-saved">{node.parent === null ? 'Tópico central' : 'Tópico'}</span>
          {#if wide}<button
            class="icon-btn"
            aria-pressed={notes.docMode}
            aria-label={notes.docMode ? 'Voltar ao card' : 'Abrir como documento'}
            title={notes.docMode ? 'Voltar ao card' : 'Abrir como documento (página inteira)'}
            onclick={() => (notes.docMode = !notes.docMode)}
          >
            {#if notes.docMode}<Minimize2 size={19} />{:else}<BookOpen size={19} />{/if}
          </button>{/if}
          <ReminderPopover
            value={meta.reminderAt}
            repeat={meta.reminderRepeat}
            onchange={(v) => card?.reminder(v)}
            onrepeat={(r) => (meta.reminderRepeat = r)}
          />
          <DropdownMenu.Root>
            <DropdownMenu.Trigger class="icon-btn" aria-label="Mais opções"><MoreVertical size={19} /></DropdownMenu.Trigger>
            <DropdownMenu.Portal>
              <DropdownMenu.Content class="menu" align="end" sideOffset={6}>
                <DropdownMenu.Sub>
                  <DropdownMenu.SubTrigger class="menu-item"><CircleDashed size={16} /><span class="grow">Status</span><ChevronRight size={16} /></DropdownMenu.SubTrigger>
                  <DropdownMenu.SubContent class="menu" sideOffset={4}>
                    {#each ['todo', 'doing', 'done'] as const as st (st)}
                      <DropdownMenu.Item class="menu-item" onSelect={() => card?.status(st)}>
                        <i class="menu-swatch" style:background={STATUS[st].color}></i><span class="grow">{STATUS[st].label}</span>
                        {#if hasStatus(meta) && statusOf(meta) === st}<span class="menu-check"><Check size={15} /></span>{/if}
                      </DropdownMenu.Item>
                    {/each}
                    {#if meta.statusOn && meta.reminderAt == null}
                      <DropdownMenu.Separator class="menu-sep" />
                      <DropdownMenu.Item class="menu-item" onSelect={() => card?.status('none')}><span class="grow">Sem status</span></DropdownMenu.Item>
                    {/if}
                  </DropdownMenu.SubContent>
                </DropdownMenu.Sub>
                <DropdownMenu.Item class="menu-item" onSelect={() => card?.chooseEmoji()}><SmilePlus size={16} />{meta.emoji ? 'Mudar emoji de capa' : 'Emoji de capa'}</DropdownMenu.Item>
                {#if node.parent !== null}
                  <DropdownMenu.Separator class="menu-sep" />
                  <DropdownMenu.Item class="menu-item danger" onSelect={() => void removeTopic()}><Trash2 size={16} />Apagar tópico</DropdownMenu.Item>
                {/if}
              </DropdownMenu.Content>
            </DropdownMenu.Portal>
          </DropdownMenu.Root>
        </div>
      {/snippet}
    </NoteEditor>
  {/if}
</CardDialog>

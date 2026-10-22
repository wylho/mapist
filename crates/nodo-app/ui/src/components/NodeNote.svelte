<script lang="ts">
  // Um card aberto: o do Atrio, o mesmo do Ideario, na mesma janela (`CardDialog`: diálogo na janela larga, tela
  // cheia na estreita, modo documento). Serve ao tópico (título = texto do nó; a cor do card é a cor do ramo) e à
  // nota solta (título, cor de nota e campos dela). O texto vira o Markdown no núcleo; emoji, etiquetas, lembrete e
  // status vão junto. Salva sozinho. Fotos, arquivos e gravações vão no `.nodo`; categoria e prévia de link ficam
  // desligadas.
  import { onDestroy, untrack } from 'svelte'
  import { DropdownMenu } from 'bits-ui'
  import { Trash2 } from '@lucide/svelte'
  import {
    CardDialog,
    CardTopBar,
    emptyMeta,
    features,
    type Attachment,
    type CardHost,
    type CardMeta,
    type TagCount,
  } from '@wylho/atrio'
  import { NoteEditor } from '@wylho/atrio/editor'
  import { AttachmentViewer } from '@wylho/atrio/viewer'
  import { api, attachments, hasHost, isPreview, type CardFields, type FlatNode, type NoteCard } from '../lib/api'
  import { doc, nodeColorOf, noteColorOf } from '../lib/document.svelte'
  import { openLink } from '../lib/markdown'
  import { notes, type NoteRef } from '../lib/notes.svelte'

  let { target, autofocus = false, onclose }: { target: NoteRef; autofocus?: boolean; onclose: () => void } = $props()

  // Fotos, arquivos, câmera e gravador entram no `.nodo` (um zip com os anexos). Categoria é só do Ideario; a prévia
  // de link fica desligada até o usuário decidir sobre buscar a página na internet.
  const FEATURES = features({ category: false, links: false })

  const id = untrack(() => target.id)
  const free = untrack(() => target.kind === 'free')

  /** O que está aberto, como o núcleo tem agora (tópico ou nota solta). */
  const current = (): FlatNode | NoteCard | undefined => (free ? doc.freeNote(id) : (doc.node(id) ?? undefined))
  const textOf = (x: FlatNode | NoteCard) => ('note' in x ? x.note : x.text)
  const isRoot = untrack(() => !free && doc.node(id)?.parent === null)

  // Modo documento só na janela larga, como no Ideario.
  const wideQuery = matchMedia('(min-width: 960px)')
  let wide = $state(wideQuery.matches)
  $effect(() => {
    const on = (e: MediaQueryListEvent) => (wide = e.matches)
    wideQuery.addEventListener('change', on)
    return () => wideQuery.removeEventListener('change', on)
  })

  function metaOf(n: FlatNode | NoteCard): CardMeta {
    const c = n.card ?? {}
    return emptyMeta({
      title: 'note' in n ? n.text : n.title,
      color: 'note' in n ? noteColorOf(n.color) : ((n.color ?? 'none') as CardMeta['color']),
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

  const meta = $state<CardMeta>(untrack(() => metaOf(current()!)))
  let saved = untrack(() => keyOf(meta))
  let content = $state<unknown>(null)
  let lastNote = untrack(() => textOf(current()!))
  const readDoc = () => (free ? api.freeNoteDoc(id) : api.noteDoc(id))
  let card = $state<ReturnType<typeof NoteEditor> | undefined>(undefined)
  let bodyTimer: ReturnType<typeof setTimeout> | undefined
  let metaTimer: ReturnType<typeof setTimeout> | undefined
  let pendingBody: unknown = null

  void readDoc().then((d) => (content = d))

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
    const n = current()
    if (!n) return
    const was = metaOf(n)
    saved = key
    if (free) {
      await doc.setFreeCard(id, meta.title, meta.color === 'none' ? null : meta.color, fieldsOf(meta))
      return
    }
    const title = meta.title.trim()
    if (title && title !== (n as FlatNode).text) await doc.setText(id, title)
    if (meta.color !== was.color) await doc.setColor(id, nodeColorOf(meta.color))
    if (JSON.stringify(fieldsOf(meta)) !== JSON.stringify(fieldsOf(was))) await doc.setCard(id, fieldsOf(meta))
  }

  // O núcleo mudou por fora (desfazer, MCP): o card acompanha, se não houver nada nosso por gravar.
  $effect(() => {
    const n = current()
    if (!n) return
    untrack(() => {
      const fresh = metaOf(n)
      if (!metaTimer && keyOf(fresh) !== saved && keyOf(fresh) !== keyOf(meta)) {
        Object.assign(meta, fresh)
        saved = keyOf(fresh)
      }
      if (!bodyTimer && textOf(n) !== lastNote) {
        lastNote = textOf(n)
        void readDoc().then((d) => card?.getEditor()?.commands.setContent(d as never, { emitUpdate: false }))
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
    await (free ? doc.setFreeNoteDoc(id, body) : doc.setNoteDoc(id, body))
    const n = current()
    if (n) lastNote = textOf(n)
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
    if (free) return void (await doc.removeFreeNote(id))
    doc.selected = id
    await doc.remove()
  }

  /** Etiquetas usadas no mapa, para sugerir. */
  function knownTags(): TagCount[] {
    const count = new Map<string, number>()
    for (const n of doc.snapshot.nodes) for (const t of n.card?.tags ?? []) count.set(t, (count.get(t) ?? 0) + 1)
    return [...count].map(([name, n]) => ({ name, count: n }))
  }

  let viewing = $state<Attachment | null>(null)

  const host: CardHost = {
    tags: knownTags,
    mediaUrl: (h) => attachments.url(h),
    imageUrl: (h) => attachments.url(h),
    importFile: (f, name) => attachments.importFile(f, name),
    importPath: (p) => attachments.importPath(p),
    attachments: (hs) => attachments.list(hs),
    download: (a) => void attachments.download(a),
    // O visualizador do Atrio, o mesmo do Ideario (foto, PDF, vídeo, áudio); o resto se baixa de lá.
    openAttachment: (a) => (viewing = a),
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
        <CardTopBar
          {meta}
          {card}
          onback={close}
          backLabel="Voltar ao mapa"
          {wide}
          docMode={notes.docMode}
          ondoc={() => (notes.docMode = !notes.docMode)}
        >
          {#snippet label()}{free ? 'Nota' : isRoot ? 'Tópico central' : 'Tópico'}{/snippet}
          {#snippet menuEnd()}
            {#if !isRoot}
              <DropdownMenu.Separator class="menu-sep" />
              <DropdownMenu.Item class="menu-item danger" onSelect={() => void removeTopic()}><Trash2 size={16} />{free ? 'Apagar nota' : 'Apagar tópico'}</DropdownMenu.Item>
            {/if}
          {/snippet}
        </CardTopBar>
      {/snippet}
    </NoteEditor>
  {/if}
</CardDialog>

<AttachmentViewer
  att={viewing}
  imageUrl={(h) => attachments.url(h)}
  mediaUrl={(h) => attachments.url(h)}
  onclose={() => (viewing = null)}
  ondownload={(a) => void attachments.download(a)}
/>

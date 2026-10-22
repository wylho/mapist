// Nota em Markdown ↔ documento do editor do card, só para o preview no navegador. No app quem converte é o núcleo,
// com as regras do Ideario (`atrio::markdown`); aqui basta o que o mapa de exemplo usa: parágrafos, títulos, listas,
// tarefas, negrito/itálico, fotos e arquivos (`ideario://att/<hash>`, como o núcleo escreve).

type Mark = { type: string }
type Inline = { type: 'text'; text: string; marks?: Mark[] }
type Block = { type: string; attrs?: Record<string, unknown>; content?: (Block | Inline)[] }

function inline(md: string): Inline[] {
  const out: Inline[] = []
  for (const part of md.split(/(\*\*[^*]+\*\*|\*[^*]+\*)/)) {
    if (!part) continue
    if (/^\*\*.+\*\*$/.test(part)) out.push({ type: 'text', text: part.slice(2, -2), marks: [{ type: 'bold' }] })
    else if (/^\*.+\*$/.test(part)) out.push({ type: 'text', text: part.slice(1, -1), marks: [{ type: 'italic' }] })
    else out.push({ type: 'text', text: part })
  }
  return out
}

const para = (text: string): Block => ({ type: 'paragraph', ...(text ? { content: inline(text) } : {}) })

export function markdownToDoc(md: string): { type: 'doc'; content: Block[] } {
  const content: Block[] = []
  let list = null as Block | null
  for (const line of md.split('\n')) {
    const images = /^\s*(?:!\[[^\]]*\]\(ideario:\/\/att\/[0-9a-f]+\)\s*)+$/.test(line)
      ? [...line.matchAll(/!\[[^\]]*\]\(ideario:\/\/att\/([0-9a-f]+)\)/g)].map((m) => m[1])
      : []
    const file = /^\s*\[📎\s*(.*?)\]\(ideario:\/\/att\/([0-9a-f]+)\)\s*$/.exec(line)
    if (images.length || file) {
      list = null
      if (file) content.push({ type: 'noteFile', attrs: { hash: file[2] } })
      else if (images.length === 1) content.push({ type: 'noteImage', attrs: { hash: images[0] } })
      else content.push({ type: 'imageRow', content: images.map((hash) => ({ type: 'noteImage', attrs: { hash } })) })
      continue
    }
    const task = /^\s*[-*] \[([ xX])\] (.*)$/.exec(line)
    const bullet = /^\s*[-*] (.*)$/.exec(line)
    const ordered = /^\s*\d+\. (.*)$/.exec(line)
    const heading = /^(#{1,3}) (.*)$/.exec(line)
    const kind = task ? 'taskList' : bullet ? 'bulletList' : ordered ? 'orderedList' : null
    if (kind) {
      if (list?.type !== kind) content.push((list = { type: kind, content: [] }))
      const text = (task?.[2] ?? bullet?.[1] ?? ordered?.[1] ?? '').trim()
      list.content?.push(
        task
          ? { type: 'taskItem', attrs: { checked: task[1] !== ' ' }, content: [para(text)] }
          : { type: 'listItem', content: [para(text)] },
      )
      continue
    }
    list = null
    if (heading) content.push({ type: 'heading', attrs: { level: heading[1].length }, content: inline(heading[2]) })
    else if (line.trim()) content.push(para(line.trim()))
  }
  return { type: 'doc', content: content.length ? content : [para('')] }
}

function text(n: Block | Inline): string {
  if (n.type === 'text') {
    const t = (n as Inline).text
    const marks = (n as Inline).marks?.map((m) => m.type) ?? []
    return marks.includes('bold') ? `**${t}**` : marks.includes('italic') ? `*${t}*` : t
  }
  return ((n as Block).content ?? []).map(text).join('')
}

export function docToMarkdown(doc: unknown, name: (hash: string) => string | undefined = () => undefined): string {
  const blocks = ((doc as Block | null)?.content ?? []) as Block[]
  const out: string[] = []
  const image = (b: Block) => `![${name(String(b.attrs?.hash)) ?? 'foto'}](ideario://att/${b.attrs?.hash})`
  for (const b of blocks) {
    if (b.type === 'noteImage') out.push(image(b))
    else if (b.type === 'imageRow') out.push(((b.content ?? []) as Block[]).map(image).join(' '))
    else if (b.type === 'noteFile') out.push(`[📎 ${name(String(b.attrs?.hash)) ?? 'arquivo'}](ideario://att/${b.attrs?.hash})`)
    else if (b.type === 'heading') out.push(`${'#'.repeat(Number(b.attrs?.level ?? 1))} ${text(b)}`)
    else if (b.type === 'taskList' || b.type === 'bulletList' || b.type === 'orderedList')
      out.push(
        (b.content ?? [])
          .map((it, i) => {
            const item = it as Block
            const mark =
              b.type === 'taskList' ? `- [${item.attrs?.checked ? 'x' : ' '}]` : b.type === 'orderedList' ? `${i + 1}.` : '-'
            return `${mark} ${text(item)}`
          })
          .join('\n'),
      )
    else if (text(b).trim()) out.push(text(b))
  }
  return out.join('\n\n')
}

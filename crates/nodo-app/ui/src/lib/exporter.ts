// Exportação: PNG, SVG e PDF saem do mesmo desenho da tela (render.ts), sem seleção nem realce; Markdown e
// OPML vêm do núcleo (nodo_core::export), os mesmos que o Claude exporta pelo MCP. Gravar no disco passa pelo Rust
// (gravação atômica); na prévia, vira um download do navegador.
import { api, hasHost, type Snapshot } from './api'
import { draw, readThemeColors, resolveColor } from './render'
import { cardKey, paintCards, type CardPaint } from './domcard'
import { branchColors, mapTheme } from './maptheme'
import { measureNode } from './measure'
import { prefs } from './prefs.svelte'
import { theme } from './theme.svelte'
import { SvgContext } from './svgctx'
import { t } from '@wylho/atrio'

export type ExportFormat = 'png' | 'svg' | 'pdf' | 'markdown' | 'opml'
export type Orientation = 'auto' | 'portrait' | 'landscape'
export type Paper = 'a4' | 'letter'
/** Os cards na imagem: inteiros (como na tela) ou só o título de cada tópico. */
export type CardsMode = 'full' | 'title'

export interface ExportOptions {
  /** PNG: quantas vezes o tamanho real (2 é nítido em tela Retina). */
  scale: number
  /** PNG e SVG: fundo do mapa ou transparente. */
  background: boolean
  orientation: Orientation
  paper: Paper
  cards: CardsMode
  /** Abrir todos os ramos recolhidos na imagem (o mapa na tela continua como está). */
  unfold: boolean
}

// Rótulos em português (a chave da tradução): a tela passa cada um por `t` na hora de mostrar.
export const FORMATS: { id: ExportFormat; label: string; ext: string; hint: string; mime: string }[] = [
  { id: 'png', label: 'Imagem PNG', ext: 'png', hint: 'Para mensagens, slides e redes', mime: 'image/png' },
  { id: 'svg', label: 'SVG', ext: 'svg', hint: 'Vetor aberto: amplia sem perder e edita no Figma, Inkscape…', mime: 'image/svg+xml' },
  { id: 'pdf', label: 'PDF para imprimir', ext: 'pdf', hint: 'O mapa inteiro numa página, em alta resolução', mime: 'application/pdf' },
  { id: 'markdown', label: 'Markdown', ext: 'md', hint: 'Lista recuada com as notas, para Obsidian, Notion…', mime: 'text/markdown' },
  { id: 'opml', label: 'OPML', ext: 'opml', hint: 'Lista de tópicos que o MindNode, o XMind e apps de listas abrem', mime: 'text/x-opml' },
]

const MARGIN = 40

/**
 * O mapa inteiro com uma margem, em unidades de mundo. Com os cards lidos da tela, a nota solta entra com a altura de
 * verdade (o card cresce com o texto; a altura guardada é a do último arraste).
 */
export function exportFrame(s: Snapshot, cards?: Map<string, CardPaint>) {
  const b = { ...s.bounds }
  for (const f of cards ? s.notes : []) {
    const p = cards?.get(cardKey('f', f.id))
    if (p) b.max_y = Math.max(b.max_y, f.y + p.height)
  }
  return { width: b.max_x - b.min_x + MARGIN * 2, height: b.max_y - b.min_y + MARGIN * 2, x: MARGIN - b.min_x, y: MARGIN - b.min_y }
}

/** Orientação que faz o mapa caber maior: paisagem para mapas largos, retrato para os altos. */
export function pickOrientation(s: Snapshot, o: Orientation, cards?: Map<string, CardPaint>): 'portrait' | 'landscape' {
  if (o !== 'auto') return o
  const f = exportFrame(s, cards)
  return f.width >= f.height ? 'landscape' : 'portrait'
}

/** Os cards do mapa como a tela os mostra (o próprio `NoteCard` do Atrio, lido em `domcard.ts`). */
export function readCards(s: Snapshot): Promise<Map<string, CardPaint>> {
  const c = readThemeColors()
  const branch = branchColors(s.nodes, mapTheme(s.style), theme.mode, c.nodeColors, (x) => resolveColor(x, c))
  return paintCards(s, branch)
}

function paint(ctx: unknown, s: Snapshot, scale: number, cards: Map<string, CardPaint>) {
  const f = exportFrame(s, cards)
  draw({
    cards,
    snapshot: s, colors: readThemeColors(), view: { scale, x: f.x * scale, y: f.y * scale },
    selected: null, hovered: null, matches: new Set(), width: f.width * scale, height: f.height * scale, dpr: 1,
    linkStyle: prefs.linkStyle, mode: theme.mode, paintCards: true, ...({ ctx } as Record<string, unknown>),
  } as never)
}

function background(): string {
  return readThemeColors().stage
}

/** O mapa num canvas, `scale` vezes o tamanho real, com ou sem fundo. */
export function renderCanvas(s: Snapshot, scale: number, withBackground: boolean, cards: Map<string, CardPaint>): HTMLCanvasElement {
  const f = exportFrame(s, cards)
  const c = document.createElement('canvas')
  c.width = Math.ceil(f.width * scale)
  c.height = Math.ceil(f.height * scale)
  const ctx = c.getContext('2d')!
  paint(ctx, s, scale, cards)
  if (withBackground) {
    // O desenho limpa a área antes de pintar: o fundo entra por baixo depois.
    ctx.setTransform(1, 0, 0, 1, 0, 0)
    ctx.globalCompositeOperation = 'destination-over'
    ctx.fillStyle = background()
    ctx.fillRect(0, 0, c.width, c.height)
    ctx.globalCompositeOperation = 'source-over'
  }
  return c
}

export function renderSvg(s: Snapshot, withBackground: boolean, cards: Map<string, CardPaint>): string {
  const f = exportFrame(s, cards)
  const ctx = new SvgContext()
  paint(ctx, s, 1, cards)
  return ctx.toSvg(f.width, f.height, withBackground ? background() : null)
}

/** PDF de uma página: o mapa cabe inteiro, centrado, com margem de 10 mm, desenhado a ~300 dpi. */
export async function renderPdf(s: Snapshot, o: Pick<ExportOptions, 'orientation' | 'paper'>, cards: Map<string, CardPaint>): Promise<Blob> {
  // Carregado só aqui: a biblioteca de PDF não pesa na abertura do app.
  const { jsPDF } = await import('jspdf')
  const orientation = pickOrientation(s, o.orientation, cards)
  const pdf = new jsPDF({ orientation, unit: 'mm', format: o.paper, compress: true })
  const pw = pdf.internal.pageSize.getWidth()
  const ph = pdf.internal.pageSize.getHeight()
  const f = exportFrame(s, cards)
  const k = Math.min((pw - 20) / f.width, (ph - 20) / f.height)
  const w = f.width * k
  const h = f.height * k
  const px = (w / 25.4) * 300
  const scale = Math.max(1, Math.min(6, px / f.width))
  const image = renderCanvas(s, scale, true, cards).toDataURL('image/png')
  pdf.setProperties({ title: s.title, creator: 'Nodo' })
  pdf.addImage(image, 'PNG', (pw - w) / 2, (ph - h) / 2, w, h, undefined, 'FAST')
  return pdf.output('blob')
}

/**
 * O mapa como a exportação pediu, sem mudar o documento: com `unfold`, todos os ramos abertos; com só o título, cada
 * tópico com card vira o tópico comum (a pílula com o nome) e o mapa é arrumado nesse tamanho, sem buracos entre os
 * ramos (as notas soltas ficam de fora). O núcleo arruma e devolve tudo como estava.
 */
export async function exportView(s: Snapshot, o: Pick<ExportOptions, 'cards' | 'unfold'>): Promise<Snapshot> {
  if (!o.unfold && o.cards === 'full') return s
  const theme = mapTheme(s.style)
  const plain = (n: Snapshot['nodes'][number]) => ({ ...n, note: '', card: {}, view: undefined })
  // Abrir tudo mostra tópicos que a tela não mediu: primeiro o mapa aberto, depois as medidas de todos.
  const open = o.unfold ? await api.exportLayout([], true) : s
  const sizes = open.nodes.map((n) => ({ id: n.id, ...(o.cards === 'title' ? measureNode(plain(n), theme, true) : measureNode(n, theme)) }))
  const laid = await api.exportLayout(sizes, o.unfold)
  if (o.cards === 'full') return laid
  // Sem as notas soltas, a moldura é só a dos tópicos.
  const nodes = laid.nodes.map(plain)
  const bounds = {
    min_x: Math.min(...nodes.map((n) => n.x)),
    min_y: Math.min(...nodes.map((n) => n.y)),
    max_x: Math.max(...nodes.map((n) => n.x + n.width)),
    max_y: Math.max(...nodes.map((n) => n.y + n.height)),
  }
  return { ...laid, nodes, notes: [], bounds }
}

/** Os bytes do arquivo no formato pedido. */
export async function exportBytes(format: ExportFormat, source: Snapshot, o: ExportOptions): Promise<Uint8Array> {
  const text = (t: string) => new TextEncoder().encode(t)
  const image = format === 'png' || format === 'svg' || format === 'pdf'
  const s = image ? await exportView(source, o) : source
  const cards = image && o.cards === 'full' ? await readCards(s) : new Map<string, CardPaint>()
  switch (format) {
    case 'png': {
      const blob = await new Promise<Blob | null>((ok) => renderCanvas(s, o.scale, o.background, cards).toBlob(ok, 'image/png'))
      if (!blob) throw new Error(t('não foi possível gerar a imagem'))
      return new Uint8Array(await blob.arrayBuffer())
    }
    case 'svg':
      return text(renderSvg(s, o.background, cards))
    case 'pdf':
      return new Uint8Array(await (await renderPdf(s, o, cards)).arrayBuffer())
    default:
      return text(await api.exportText(format))
  }
}

/** Pergunta onde salvar e grava. Devolve o caminho (ou o nome, na prévia), ou null se a pessoa cancelou. */
export async function exportTo(format: ExportFormat, s: Snapshot, o: ExportOptions): Promise<string | null> {
  const f = FORMATS.find((x) => x.id === format)!
  const base = (s.title || 'mapa').replace(/[\\/:*?"<>|]+/g, ' ').trim() || 'mapa'
  const name = `${base}.${f.ext}`
  const bytes = await exportBytes(format, s, o)
  if (hasHost) {
    const { save } = await import('@tauri-apps/plugin-dialog')
    const path = await save({ defaultPath: name, filters: [{ name: t(f.label), extensions: [f.ext] }] })
    if (!path) return null
    await api.writeExport(path, bytes)
    return path
  }
  const url = URL.createObjectURL(new Blob([bytes as BlobPart], { type: f.mime }))
  const a = document.createElement('a')
  a.href = url
  a.download = name
  // Fora da página o navegador ignora o nome do arquivo.
  document.body.append(a)
  a.click()
  a.remove()
  setTimeout(() => URL.revokeObjectURL(url), 2000)
  return name
}

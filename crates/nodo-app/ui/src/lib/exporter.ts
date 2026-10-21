// Exportação: PNG, SVG e PDF saem do mesmo desenho da tela (render.ts), sem seleção nem realce; Markdown e
// OPML vêm do núcleo (nodo_core::export), os mesmos que o Claude exporta pelo MCP. Gravar no disco passa pelo Rust
// (gravação atômica); na prévia, vira um download do navegador.
import { api, hasHost, type Snapshot } from './api'
import { draw, readThemeColors } from './render'
import { prefs } from './prefs.svelte'
import { theme } from './theme.svelte'
import { SvgContext } from './svgctx'

export type ExportFormat = 'png' | 'svg' | 'pdf' | 'markdown' | 'opml'
export type Orientation = 'auto' | 'portrait' | 'landscape'
export type Paper = 'a4' | 'letter'

export interface ExportOptions {
  /** PNG: quantas vezes o tamanho real (2 é nítido em tela Retina). */
  scale: number
  /** PNG e SVG: fundo do mapa ou transparente. */
  background: boolean
  orientation: Orientation
  paper: Paper
}

export const FORMATS: { id: ExportFormat; label: string; ext: string; hint: string; mime: string }[] = [
  { id: 'png', label: 'Imagem PNG', ext: 'png', hint: 'Para mensagens, slides e redes', mime: 'image/png' },
  { id: 'svg', label: 'SVG', ext: 'svg', hint: 'Vetor aberto: amplia sem perder e edita no Figma, Inkscape…', mime: 'image/svg+xml' },
  { id: 'pdf', label: 'PDF para imprimir', ext: 'pdf', hint: 'O mapa inteiro numa página, em alta resolução', mime: 'application/pdf' },
  { id: 'markdown', label: 'Markdown', ext: 'md', hint: 'Lista recuada com as notas, para Obsidian, Notion…', mime: 'text/markdown' },
  { id: 'opml', label: 'OPML', ext: 'opml', hint: 'Lista de tópicos que o MindNode, o XMind e apps de listas abrem', mime: 'text/x-opml' },
]

const MARGIN = 40

/** O mapa inteiro com uma margem, em unidades de mundo. */
export function exportFrame(s: Snapshot) {
  const b = s.bounds
  return { width: b.max_x - b.min_x + MARGIN * 2, height: b.max_y - b.min_y + MARGIN * 2, x: MARGIN - b.min_x, y: MARGIN - b.min_y }
}

/** Orientação que faz o mapa caber maior: paisagem para mapas largos, retrato para os altos. */
export function pickOrientation(s: Snapshot, o: Orientation): 'portrait' | 'landscape' {
  if (o !== 'auto') return o
  const f = exportFrame(s)
  return f.width >= f.height ? 'landscape' : 'portrait'
}

function paint(ctx: unknown, s: Snapshot, scale: number) {
  const f = exportFrame(s)
  draw({
    snapshot: s, colors: readThemeColors(), view: { scale, x: f.x * scale, y: f.y * scale },
    selected: null, hovered: null, matches: new Set(), width: f.width * scale, height: f.height * scale, dpr: 1,
    linkStyle: prefs.linkStyle, mode: theme.mode, paintCards: true, ...({ ctx } as Record<string, unknown>),
  } as never)
}

function background(): string {
  return readThemeColors().stage
}

/** O mapa num canvas, `scale` vezes o tamanho real, com ou sem fundo. */
export function renderCanvas(s: Snapshot, scale: number, withBackground: boolean): HTMLCanvasElement {
  const f = exportFrame(s)
  const c = document.createElement('canvas')
  c.width = Math.ceil(f.width * scale)
  c.height = Math.ceil(f.height * scale)
  const ctx = c.getContext('2d')!
  paint(ctx, s, scale)
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

export function renderSvg(s: Snapshot, withBackground: boolean): string {
  const f = exportFrame(s)
  const ctx = new SvgContext()
  paint(ctx, s, 1)
  return ctx.toSvg(f.width, f.height, withBackground ? background() : null)
}

/** PDF de uma página: o mapa cabe inteiro, centrado, com margem de 10 mm, desenhado a ~300 dpi. */
export async function renderPdf(s: Snapshot, o: Pick<ExportOptions, 'orientation' | 'paper'>): Promise<Blob> {
  // Carregado só aqui: a biblioteca de PDF não pesa na abertura do app.
  const { jsPDF } = await import('jspdf')
  const orientation = pickOrientation(s, o.orientation)
  const pdf = new jsPDF({ orientation, unit: 'mm', format: o.paper, compress: true })
  const pw = pdf.internal.pageSize.getWidth()
  const ph = pdf.internal.pageSize.getHeight()
  const f = exportFrame(s)
  const k = Math.min((pw - 20) / f.width, (ph - 20) / f.height)
  const w = f.width * k
  const h = f.height * k
  const px = (w / 25.4) * 300
  const scale = Math.max(1, Math.min(6, px / f.width))
  const image = renderCanvas(s, scale, true).toDataURL('image/png')
  pdf.setProperties({ title: s.title, creator: 'Nodo' })
  pdf.addImage(image, 'PNG', (pw - w) / 2, (ph - h) / 2, w, h, undefined, 'FAST')
  return pdf.output('blob')
}

/** Os bytes do arquivo no formato pedido. */
export async function exportBytes(format: ExportFormat, s: Snapshot, o: ExportOptions): Promise<Uint8Array> {
  const text = (t: string) => new TextEncoder().encode(t)
  switch (format) {
    case 'png': {
      const blob = await new Promise<Blob | null>((ok) => renderCanvas(s, o.scale, o.background).toBlob(ok, 'image/png'))
      if (!blob) throw new Error('não foi possível gerar a imagem')
      return new Uint8Array(await blob.arrayBuffer())
    }
    case 'svg':
      return text(renderSvg(s, o.background))
    case 'pdf':
      return new Uint8Array(await (await renderPdf(s, o)).arrayBuffer())
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
    const path = await save({ defaultPath: name, filters: [{ name: f.label, extensions: [f.ext] }] })
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

// Desenho do mapa no canvas.
//
// Recebe o snapshot e as cores do tema ativo. Nada aqui conhece o Rust nem o
// estado da aplicação: é uma função de (dados, tema, viewport) para pixels.

import type { Snapshot } from './api'
import type { FlatNode } from './api'
import { fitText } from './measure'
import { levelFont, uiFont } from './font'
import { NODE_COLOR_TOKENS } from './palettes'
import { branchColors, levelStyle, lineWidth, mapTheme, softFill, textOn, type LevelStyle, type MapTheme } from './maptheme'

export interface Viewport {
  x: number
  y: number
  scale: number
}

export interface ThemeColors {
  stage: string
  surface: string
  raised: string
  fg: string
  muted: string
  line: string
  accent: string
  onAccent: string
  cardEdge: string
  /** Cores de nó do tema, na ordem areia, sálvia, céu, rosa, lilás, manteiga. */
  nodeColors: string[]
}

/**
 * Traçado das linhas entre pai e filho: `curved` é a curva Bézier solta;
 * `orthogonal` anda em degrau (horizontal, vertical, horizontal) com cantos
 * arredondados.
 */
export type LinkStyle = 'curved' | 'orthogonal'

export interface DrawOptions {
  snapshot: Snapshot
  linkStyle?: LinkStyle
  /** Claro ou escuro: os recheios suaves e a paleta cinza dependem dele. */
  mode?: 'light' | 'dark'
  colors: ThemeColors
  view: Viewport
  selected: number | null
  hovered: number | null
  matches: Set<number>
  width: number
  height: number
  dpr: number
}

/**
 * Converte um token de cor de nó (`--n-sage`) no valor concreto do tema.
 *
 * A fonte da verdade da ordem dos tokens é `NODE_COLOR_TOKENS` em palettes.ts;
 * aqui só se consulta, para que a lista não exista em dois lugares e divirja.
 * Cores hex passam direto, exatamente como vieram de um `.minder`.
 */
export function resolveColor(token: string | null, colors: ThemeColors): string {
  if (!token) return colors.surface
  const i = NODE_COLOR_TOKENS.indexOf(token as (typeof NODE_COLOR_TOKENS)[number])
  if (i >= 0) return colors.nodeColors[i]
  return token
}

export function draw(o: DrawOptions) {
  const { snapshot, colors, view, selected, hovered, matches, width, height, dpr } = o
  const linkStyle = o.linkStyle ?? 'curved'
  const canvas = o as DrawOptions & { ctx?: CanvasRenderingContext2D }
  const ctx = canvas.ctx
  if (!ctx) return

  // Um único ponto onde o zoom entra: daqui para baixo tudo é coordenada de
  // mundo, então geometria, traço e texto escalam juntos e nunca divergem.
  ctx.setTransform(dpr * view.scale, 0, 0, dpr * view.scale, dpr * view.x, dpr * view.y)
  ctx.clearRect(
    -view.x / view.scale,
    -view.y / view.scale,
    width / view.scale,
    height / view.scale,
  )

  if (snapshot.nodes.length === 0) {
    ctx.fillStyle = colors.muted
    ctx.font = uiFont(14)
    ctx.textAlign = 'center'
    ctx.textBaseline = 'middle'
    ctx.fillText('Nenhum mapa aberto — use Novo para começar', width / 2, height / 2)
    return
  }

  const byId = new Map(snapshot.nodes.map((n) => [n.id, n]))
  const theme = mapTheme(snapshot.style)
  const mode = o.mode ?? 'light'
  const branch = branchColors(snapshot.nodes, theme, mode, colors.nodeColors, (c) => resolveColor(c, colors))

  // Ramos primeiro, para os nós ficarem por cima.
  for (const n of snapshot.nodes) {
    if (n.parent === null) continue
    const p = byId.get(n.parent)
    if (p) drawBranch(ctx, p, n, theme, branch.get(n.id) ?? colors.muted, linkStyle)
  }

  for (const n of snapshot.nodes) {
    drawNode(ctx, n, {
      colors,
      theme,
      mode,
      branch: branch.get(n.id) ?? colors.muted,
      isSelected: n.id === selected,
      isHovered: n.id === hovered,
      isMatch: matches.has(n.id),
    })
  }
}

/**
 * Altura onde um ramo encosta no nó: no meio da caixa, ou na linha de baixo quando o nível é sublinhado
 * (o texto fica em cima da linha, como no XMind).
 */
function anchorY(n: FlatNode, style: LevelStyle): number {
  return style.underline ? n.y + n.height : n.y + n.height / 2
}

function drawBranch(ctx: CanvasRenderingContext2D, parent: FlatNode, child: FlatNode, theme: MapTheme, color: string, style: LinkStyle) {
  const left = child.side === 'left'
  const ax = left ? parent.x : parent.x + parent.width
  const ay = anchorY(parent, levelStyle(theme, parent.depth))
  const bx = left ? child.x + child.width : child.x
  const by = anchorY(child, levelStyle(theme, child.depth))

  ctx.save()
  ctx.strokeStyle = color
  ctx.lineWidth = lineWidth(theme, child.depth)
  ctx.lineCap = 'round'
  ctx.beginPath()
  traceBranch(ctx, style, ax, ay, bx, by)
  ctx.stroke()
  ctx.restore()
}
/** Raio máximo dos cantos do traçado ortogonal, em unidades de mundo. */
const CORNER_RADIUS = 14

/**
 * Traça o caminho de (ax, ay) a (bx, by) no estilo pedido. Só monta o caminho:
 * quem chama decide traço e cor. Separado para poder ser testado sem canvas.
 */
export function traceBranch(
  ctx: Pick<CanvasRenderingContext2D, 'moveTo' | 'lineTo' | 'bezierCurveTo' | 'arcTo'>,
  style: LinkStyle,
  ax: number,
  ay: number,
  bx: number,
  by: number,
) {
  ctx.moveTo(ax, ay)
  const mid = (ax + bx) / 2
  if (style === 'curved') {
    ctx.bezierCurveTo(mid, ay, mid, by, bx, by)
    return
  }
  // O canto não pode passar da metade do desnível nem da metade do trecho
  // horizontal, senão os dois arcos se cruzam. Alinhados, vira uma reta.
  const r = Math.min(CORNER_RADIUS, Math.abs(by - ay) / 2, Math.abs(mid - ax))
  if (r < 0.5) {
    ctx.lineTo(bx, by)
    return
  }
  ctx.arcTo(mid, ay, mid, by, r)
  ctx.arcTo(mid, by, bx, by, r)
  ctx.lineTo(bx, by)
}

interface NodeOpts {
  colors: ThemeColors
  theme: MapTheme
  mode: 'light' | 'dark'
  /** Cor do ramo deste nó (já resolvida). */
  branch: string
  isSelected: boolean
  isHovered: boolean
  isMatch: boolean
}

function drawNode(ctx: CanvasRenderingContext2D, n: FlatNode, o: NodeOpts) {
  const { colors, theme, mode, branch, isSelected, isHovered, isMatch } = o
  const { x, y, width: w, height: h } = n
  const isRoot = n.parent === null
  const style = levelStyle(theme, n.depth)
  const radius = Math.min(h * style.round, 18)

  const fill =
    style.fill === 'accent' ? colors.accent
    : style.fill === 'branch' ? branch
    : style.fill === 'soft' ? softFill(branch, colors.surface, mode)
    : style.fill === 'surface' ? colors.surface
    : null

  ctx.save()

  if (fill) {
    roundRect(ctx, x, y, w, h, radius)
    ctx.fillStyle = fill
    ctx.fill()
    if (isRoot) {
      // Uma sombra leve na raiz, como o cartão em destaque do Ideario.
      ctx.shadowColor = fill
      ctx.shadowBlur = 14
      ctx.shadowOffsetY = 3
      ctx.globalAlpha = 0.35
      ctx.fill()
      ctx.globalAlpha = 1
      ctx.shadowColor = 'transparent'
      ctx.shadowBlur = 0
      ctx.shadowOffsetY = 0
    }
  }

  // Contorno do estilo.
  if (style.border !== 'none') {
    roundRect(ctx, x, y, w, h, radius)
    ctx.strokeStyle = style.border === 'branch' ? branch : colors.cardEdge
    ctx.lineWidth = style.border === 'branch' ? 2 : 1
    ctx.stroke()
  } else if (fill && fill === colors.surface) {
    roundRect(ctx, x, y, w, h, radius)
    ctx.strokeStyle = colors.cardEdge
    ctx.lineWidth = 1
    ctx.stroke()
  }

  // Sublinhado: a linha do ramo continua por baixo do texto.
  if (style.underline) {
    ctx.beginPath()
    ctx.moveTo(x, y + h)
    ctx.lineTo(x + w, y + h)
    ctx.strokeStyle = branch
    ctx.lineWidth = lineWidth(theme, n.depth)
    ctx.lineCap = 'round'
    ctx.stroke()
  }

  // Seleção, busca e foco por cima de tudo, com um respiro de 3 px.
  const ring = isSelected ? 1 : isMatch ? 0.55 : isHovered ? 0.4 : 0
  if (ring > 0) {
    roundRect(ctx, x - 3, y - 3, w + 6, h + 6, radius + 3)
    ctx.strokeStyle = colors.accent
    ctx.lineWidth = isSelected ? 2 : 1.5
    ctx.globalAlpha = ring
    ctx.stroke()
    ctx.globalAlpha = 1
  }

  // Rótulo
  ctx.font = levelFont(style)
  const ink = style.text === 'branch' ? branch : textOn(fill ?? colors.stage, colors.fg, colors.onAccent)
  ctx.fillStyle = ink
  ctx.textAlign = 'center'
  ctx.textBaseline = 'middle'
  const label = fitText(n.text, w - style.padX / 2, style)
  ctx.fillText(label, x + w / 2, style.underline ? y + h / 2 - 2 : y + h / 2)

  // Marcador de nota
  if (n.note) {
    ctx.beginPath()
    ctx.arc(x + w - 8, y + 8, 2.8, 0, Math.PI * 2)
    ctx.fillStyle = ink
    ctx.globalAlpha = 0.6
    ctx.fill()
    ctx.globalAlpha = 1
  }

  // Ramo recolhido: a bolinha fica do lado onde os filhos estariam.
  if (n.has_children && n.folded) {
    ctx.beginPath()
    ctx.arc(n.side === 'left' ? x - 8 : x + w + 8, anchorY(n, style), 5, 0, Math.PI * 2)
    ctx.fillStyle = branch
    ctx.fill()
    ctx.strokeStyle = colors.stage
    ctx.lineWidth = 2
    ctx.stroke()
  }

  ctx.restore()
}

function roundRect(ctx: CanvasRenderingContext2D, x: number, y: number, w: number, h: number, r: number) {
  const rad = Math.min(r, w / 2, h / 2)
  ctx.beginPath()
  ctx.moveTo(x + rad, y)
  ctx.arcTo(x + w, y, x + w, y + h, rad)
  ctx.arcTo(x + w, y + h, x, y + h, rad)
  ctx.arcTo(x, y + h, x, y, rad)
  ctx.arcTo(x, y, x + w, y, rad)
  ctx.closePath()
}

/** Testa qual nó está sob um ponto da tela, do topo para baixo. */
export function hitTest(
  snapshot: Snapshot,
  view: Viewport,
  clientX: number,
  clientY: number,
  rect: DOMRect,
): number | null {
  const wx = (clientX - rect.left - view.x) / view.scale
  const wy = (clientY - rect.top - view.y) / view.scale
  for (let i = snapshot.nodes.length - 1; i >= 0; i--) {
    const n = snapshot.nodes[i]
    if (wx >= n.x && wx <= n.x + n.width && wy >= n.y && wy <= n.y + n.height) return n.id
  }
  return null
}

/** Lê as cores do tema ativo a partir das variáveis CSS. */
export function readThemeColors(): ThemeColors {
  const s = getComputedStyle(document.documentElement)
  const v = (name: string) => s.getPropertyValue(name).trim()
  return {
    stage: v('--stage'),
    surface: v('--surface'),
    raised: v('--raised'),
    fg: v('--fg'),
    muted: v('--muted'),
    line: v('--line'),
    accent: v('--accent'),
    onAccent: v('--on-accent'),
    cardEdge: v('--card-edge'),
    nodeColors: [v('--n-sand'), v('--n-sage'), v('--n-sky'), v('--n-rose'), v('--n-lilac'), v('--n-butter')],
  }
}

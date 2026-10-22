// Desenho do mapa no canvas.
//
// Recebe o snapshot e as cores do tema ativo. Nada aqui conhece o Rust nem o
// estado da aplicação: é uma função de (dados, tema, viewport) para pixels.

import type { Snapshot } from './api'
import type { FlatNode } from './api'
import { fitText, measureRun } from './measure'
import { CARD, isCard, layoutCard } from './cardtext'
import { fontStack, levelFont, uiFont } from './font'
import { mix, NODE_COLOR_TOKENS } from './palettes'
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
  /**
   * Pinta os nós com nota como card. Na tela não: lá o card é HTML por cima do canvas (o mesmo do Ideario), e o
   * canvas só desenha a seleção em volta. Na exportação sim, porque o PNG, o SVG e o PDF saem só do desenho.
   */
  paintCards?: boolean
}

/**
 * Converte um token de cor de nó (`--n-sage`) no valor concreto do tema.
 *
 * A fonte da verdade da ordem dos tokens é `NODE_COLOR_TOKENS` em palettes.ts;
 * aqui só se consulta, para que a lista não exista em dois lugares e divirja.
 * Cores hex passam direto, exatamente como vieram do arquivo.
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
    if (p) drawBranch(ctx, p, n, theme, branch.get(n.id) ?? colors.muted, linkStyle, snapshot.structure)
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
      paintCards: o.paintCards ?? false,
    })
  }

  // Botão de recolher/expandir no começo do ramo (não sai na exportação).
  if (!o.paintCards) {
    for (const n of snapshot.nodes) {
      if (!n.has_children || n.parent === null) continue
      drawKnob(ctx, n, {
        branch: branch.get(n.id) ?? colors.muted,
        colors,
        active: n.id === hovered || n.id === selected,
        down: snapshot.structure === 'organograma',
        style: levelStyle(theme, n.depth),
      })
    }
  }
}

/** Como o botão de recolher aparece (prévia das opções: `?fold=a|b|c`). */
export type KnobStyle = 'a' | 'b' | 'c'
export let knobStyle: KnobStyle = (() => {
  const v = typeof location === 'undefined' ? null : new URLSearchParams(location.search).get('fold')
  return v === 'b' || v === 'c' ? v : 'a'
})()

/** Centro do botão de recolher: no começo do ramo dos filhos (do lado deles, ou embaixo no organograma). */
export function knobAt(n: FlatNode, style: LevelStyle, down: boolean): { x: number; y: number } {
  if (down) return { x: n.x + n.width / 2, y: n.y + n.height + 11 }
  return { x: n.side === 'left' ? n.x - 11 : n.x + n.width + 11, y: anchorY(n, style) }
}

const KNOB_R = 8.5

function drawKnob(
  ctx: CanvasRenderingContext2D,
  n: FlatNode,
  o: { branch: string; colors: ThemeColors; active: boolean; down: boolean; style: LevelStyle },
) {
  const { x, y } = knobAt(n, o.style, o.down)
  const count = n.hidden ?? 0
  const label = count > 99 ? '99+' : String(count)
  ctx.save()
  ctx.textAlign = 'center'
  ctx.textBaseline = 'middle'
  ctx.font = `700 10px ${fontStack()}`
  if (n.folded) {
    // Recolhido: sempre à vista, na cor do ramo, com quantos tópicos estão escondidos.
    if (knobStyle === 'c') {
      const t = `+${label}`
      const w = Math.max(22, ctx.measureText(t).width + 12)
      const left = n.side === 'left' && !o.down ? x - w + KNOB_R : x - (o.down ? w / 2 : KNOB_R)
      roundRect(ctx, left, y - KNOB_R, w, KNOB_R * 2, KNOB_R)
      ctx.fillStyle = o.branch
      ctx.fill()
      ctx.fillStyle = textOn(o.branch, '#ffffff', '#16201c')
      ctx.fillText(t, left + w / 2, y + 0.5)
    } else {
      ctx.beginPath()
      ctx.arc(x, y, KNOB_R + (label.length > 1 ? 1.5 : 0), 0, Math.PI * 2)
      ctx.fillStyle = o.branch
      ctx.fill()
      ctx.strokeStyle = o.colors.stage
      ctx.lineWidth = 2
      ctx.stroke()
      ctx.fillStyle = textOn(o.branch, '#ffffff', '#16201c')
      ctx.fillText(label, x, y + 0.5)
    }
  } else if (o.active || knobStyle === 'b') {
    // Aberto: o botão de recolher aparece com o mouse em cima ou o tópico escolhido (na B, um anel discreto sempre).
    const quiet = knobStyle === 'b' && !o.active
    ctx.beginPath()
    ctx.arc(x, y, quiet ? 4 : KNOB_R, 0, Math.PI * 2)
    ctx.fillStyle = quiet ? o.branch : o.colors.surface
    ctx.fill()
    ctx.strokeStyle = o.branch
    ctx.lineWidth = 1.5
    ctx.stroke()
    if (!quiet) {
      ctx.strokeStyle = o.branch
      ctx.lineWidth = 1.8
      ctx.lineCap = 'round'
      ctx.beginPath()
      if (knobStyle === 'c') {
        // Seta para o lado do tópico: "recolher para cá".
        const dir = o.down ? 0 : n.side === 'left' ? 1 : -1
        if (o.down) {
          ctx.moveTo(x - 3.5, y + 1.8)
          ctx.lineTo(x, y - 1.8)
          ctx.lineTo(x + 3.5, y + 1.8)
        } else {
          ctx.moveTo(x - dir * 1.8, y - 3.5)
          ctx.lineTo(x + dir * 1.8, y)
          ctx.lineTo(x - dir * 1.8, y + 3.5)
        }
      } else {
        ctx.moveTo(x - 3.6, y)
        ctx.lineTo(x + 3.6, y)
      }
      ctx.stroke()
    }
  }
  ctx.restore()
}

/** O tópico cujo botão de recolher está sob o ponteiro (área de toque de ao menos 11 px na tela, em qualquer zoom). */
export function hitKnob(snapshot: Snapshot, view: Viewport, clientX: number, clientY: number, rect: DOMRect): number | null {
  const wx = (clientX - rect.left - view.x) / view.scale
  const wy = (clientY - rect.top - view.y) / view.scale
  const r = Math.max(KNOB_R + 2, 11 / view.scale)
  const theme = mapTheme(snapshot.style)
  const down = snapshot.structure === 'organograma'
  for (let i = snapshot.nodes.length - 1; i >= 0; i--) {
    const n = snapshot.nodes[i]
    if (!n.has_children || n.parent === null) continue
    const k = knobAt(n, levelStyle(theme, n.depth), down)
    if (Math.hypot(wx - k.x, wy - k.y) <= r) return n.id
  }
  return null
}

/** Recheio do card de um nó com nota: a cor do ramo bem clara, como as cores de nota do Ideario (opção A). */
export function cardFill(branch: string, surface: string, mode: 'light' | 'dark'): string {
  return mix(branch, surface, mode === 'dark' ? 0.2 : 0.22)
}

/**
 * Altura onde um ramo encosta no nó: no meio da caixa, ou na linha de baixo quando o nível é sublinhado
 * (o texto fica em cima da linha, como no XMind).
 */
function anchorY(n: FlatNode, style: LevelStyle): number {
  return style.underline ? n.y + n.height : n.y + n.height / 2
}

function drawBranch(
  ctx: CanvasRenderingContext2D,
  parent: FlatNode,
  child: FlatNode,
  theme: MapTheme,
  color: string,
  style: LinkStyle,
  structure: string | null,
) {
  ctx.save()
  ctx.strokeStyle = color
  ctx.lineWidth = lineWidth(theme, child.depth)
  ctx.lineCap = 'round'
  ctx.beginPath()
  if (structure === 'organograma') {
    // De baixo do pai até o alto do filho.
    traceBranchDown(ctx, style, parent.x + parent.width / 2, parent.y + parent.height, child.x + child.width / 2, child.y)
  } else if (structure === 'arvore') {
    // Lista recuada: desce pela margem do pai e entra no filho pela esquerda.
    traceElbow(ctx, parent.x + 14, parent.y + parent.height, child.x, anchorY(child, levelStyle(theme, child.depth)))
  } else {
    // Lado pela posição de verdade: no lógico tudo cresce para a direita, mesmo um ramo marcado à esquerda.
    const left = child.x + child.width / 2 < parent.x + parent.width / 2
    const ax = left ? parent.x : parent.x + parent.width
    const bx = left ? child.x + child.width : child.x
    traceBranch(ctx, style, ax, anchorY(parent, levelStyle(theme, parent.depth)), bx, anchorY(child, levelStyle(theme, child.depth)))
  }
  ctx.stroke()
  ctx.restore()
}

type PathCtx = Pick<CanvasRenderingContext2D, 'moveTo' | 'lineTo' | 'bezierCurveTo' | 'arcTo'>

/** O mesmo que `traceBranch`, na vertical (organograma): sai para baixo e chega por cima. */
export function traceBranchDown(ctx: PathCtx, style: LinkStyle, ax: number, ay: number, bx: number, by: number) {
  ctx.moveTo(ax, ay)
  const mid = (ay + by) / 2
  if (style === 'curved') {
    ctx.bezierCurveTo(ax, mid, bx, mid, bx, by)
    return
  }
  const r = Math.min(CORNER_RADIUS, Math.abs(bx - ax) / 2, Math.abs(mid - ay))
  if (r < 0.5) {
    ctx.lineTo(bx, by)
    return
  }
  ctx.arcTo(ax, mid, bx, mid, r)
  ctx.arcTo(bx, mid, bx, by, r)
  ctx.lineTo(bx, by)
}

/** Cotovelo da árvore recuada: desce de (ax, ay) e vira para a direita até (bx, by), com o canto arredondado. */
export function traceElbow(ctx: PathCtx, ax: number, ay: number, bx: number, by: number) {
  ctx.moveTo(ax, ay)
  const r = Math.min(CORNER_RADIUS, Math.abs(by - ay), Math.abs(bx - ax))
  if (r < 0.5) {
    ctx.lineTo(ax, by)
    ctx.lineTo(bx, by)
    return
  }
  ctx.arcTo(ax, by, bx, by, r)
  ctx.lineTo(bx, by)
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
  paintCards: boolean
}

function drawNode(ctx: CanvasRenderingContext2D, n: FlatNode, o: NodeOpts) {
  if (isCard(n)) {
    drawCard(ctx, n, o)
    return
  }
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
    const d = noteDot(n)
    ctx.beginPath()
    ctx.arc(d.x, d.y, 3.2, 0, Math.PI * 2)
    ctx.fillStyle = ink
    ctx.globalAlpha = 0.6
    ctx.fill()
    ctx.globalAlpha = 1
  }

  ctx.restore()
}

/** Um nó com nota. Seleção e recolhido sempre; o card em si só quando `paintCards` (exportação). */
function drawCard(ctx: CanvasRenderingContext2D, n: FlatNode, o: NodeOpts) {
  const { colors, mode, branch } = o
  const { x, y, width: w, height: h } = n
  ctx.save()
  if (o.paintCards) {
    roundRect(ctx, x, y, w, h, CARD.radius)
    ctx.fillStyle = cardFill(branch, colors.surface, mode)
    ctx.fill()
    ctx.strokeStyle = colors.cardEdge
    ctx.lineWidth = 1
    ctx.stroke()
    const font = fontStack()
    const lay = layoutCard(n.text, n.view?.preview ?? [], measureRun)
    ctx.textAlign = 'left'
    ctx.textBaseline = 'alphabetic'
    for (const l of lay.lines) {
      const done = l.kind === 'task' && l.done
      const quiet = done || l.kind === 'more'
      const ink = quiet ? colors.muted : colors.fg
      // Como o CSS do card: parágrafos e tópicos a 85%.
      ctx.globalAlpha = l.alpha * (l.kind === 'text' || l.kind === 'bullet' || l.kind === 'ordered' ? 0.85 : 1)
      const size = l.kind === 'title' ? CARD.titleSize : l.kind === 'code' ? 11.5 : l.kind === 'more' ? 12 : CARD.bodySize
      const weight = l.kind === 'title' ? 600 : l.kind === 'heading' ? 650 : 400
      const mono = l.kind === 'code'
      if (l.first && l.mark) {
        ctx.font = `400 ${size}px ${font}`
        ctx.fillStyle = colors.muted
        ctx.fillText(l.mark, x + l.markX, y + l.y)
      }
      if (l.first && l.kind === 'task') {
        roundRect(ctx, x + l.markX + 0.75, y + l.y - 11.25, 12.5, 12.5, 4)
        ctx.strokeStyle = ink
        ctx.lineWidth = 1.5
        ctx.stroke()
        if (l.done) {
          ctx.beginPath()
          ctx.moveTo(x + l.markX + 3.8, y + l.y - 5)
          ctx.lineTo(x + l.markX + 6.2, y + l.y - 2.6)
          ctx.lineTo(x + l.markX + 10.4, y + l.y - 8)
          ctx.stroke()
        }
      }
      ctx.font = `${weight} ${size}px ${mono ? 'ui-monospace, monospace' : font}`
      ctx.fillStyle = ink
      ctx.fillText(l.text, x + l.x, y + l.y)
      if (done) {
        // Tarefa feita: riscada, como no card.
        const tw = measureRun(l.text, size, weight)
        ctx.beginPath()
        ctx.moveTo(x + l.x, y + l.y - size * 0.3)
        ctx.lineTo(x + l.x + tw, y + l.y - size * 0.3)
        ctx.strokeStyle = ink
        ctx.lineWidth = 1
        ctx.stroke()
      }
    }
    ctx.globalAlpha = 1
  }
  const ring = o.isSelected ? 1 : o.isMatch ? 0.55 : o.isHovered ? 0.4 : 0
  if (ring > 0) {
    roundRect(ctx, x - 3, y - 3, w + 6, h + 6, CARD.radius + 3)
    ctx.strokeStyle = colors.accent
    ctx.lineWidth = o.isSelected ? 2 : 1.5
    ctx.globalAlpha = ring
    ctx.stroke()
    ctx.globalAlpha = 1
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

/** Centro da bolinha de nota de um nó, em unidades de mundo (o mesmo ponto que o desenho usa). */
export function noteDot(n: FlatNode): { x: number; y: number } {
  return { x: n.x + n.width - 8, y: n.y + 8 }
}

/** O nó cuja bolinha de nota está sob o ponteiro. A área de toque tem ao menos 9 px na tela, em qualquer zoom. */
export function hitNoteDot(snapshot: Snapshot, view: Viewport, clientX: number, clientY: number, rect: DOMRect): number | null {
  const wx = (clientX - rect.left - view.x) / view.scale
  const wy = (clientY - rect.top - view.y) / view.scale
  const r = Math.max(7, 9 / view.scale)
  for (let i = snapshot.nodes.length - 1; i >= 0; i--) {
    const n = snapshot.nodes[i]
    if (!n.note || isCard(n)) continue
    const d = noteDot(n)
    if (Math.hypot(wx - d.x, wy - d.y) <= r) return n.id
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

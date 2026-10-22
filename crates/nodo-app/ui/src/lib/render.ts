// Desenho do mapa no canvas.
//
// Recebe o snapshot e as cores do tema ativo. Nada aqui conhece o Rust nem o
// estado da aplicação: é uma função de (dados, tema, viewport) para pixels.

import type { Snapshot } from './api'
import type { FlatNode } from './api'
import { wrapText } from './measure'
import { t } from '@wylho/atrio'
import { CARD, isCard } from './cardtext'
import { cardKey, type CardPaint, type Radii } from './domcard'
import { fontStack, levelFont, uiFont } from './font'
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
  /** Os tópicos do grupo escolhido (arrastar um retângulo, ⇧ + clique): todos com a moldura de escolhido. */
  picked?: Set<number>
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
  /** Os cards lidos da tela (`domcard.ts`), para a exportação repetir o card exato; sem eles, o card não é pintado. */
  cards?: Map<string, CardPaint>
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
  const picked = o.picked ?? new Set<number>()
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
    ctx.fillText(t('Nenhum mapa aberto — use Novo para começar'), width / 2, height / 2)
    return
  }

  const byId = new Map(snapshot.nodes.map((n) => [n.id, n]))
  const theme = mapTheme(snapshot.style)
  const mode = o.mode ?? 'light'
  const branch = branchColors(snapshot.nodes, theme, mode, colors.nodeColors, (c) => resolveColor(c, colors))

  // Tabela: as células no lugar dos ramos. Cada uma na cor que o tópico teria; a mais funda que não tem célula (o que
  // fica listado dentro de uma célula da matriz) usa a cor da célula onde está.
  const table = isTable(snapshot.structure)
  const tableBg = new Map<number, string>()
  if (table) {
    for (const n of snapshot.nodes) {
      // Título e cabeçalhos (raiz e tópicos principais) na cor do tema; as células de dentro na cor suave do ramo.
      const b = branch.get(n.id) ?? colors.muted
      const own = !n.cell ? null
        : n.depth <= 1 ? fillOf(levelStyle(theme, n.depth).fill, colors, b, mode) ?? colors.surface
        : softFill(b, colors.surface, mode)
      tableBg.set(n.id, own ?? tableBg.get(n.parent ?? -1) ?? colors.surface)
      if (!n.cell) continue
      const [cx, cy, cw, ch] = n.cell
      roundRect(ctx, cx + 1, cy + 1, cw - 2, ch - 2, 6)
      ctx.fillStyle = tableBg.get(n.id)!
      ctx.fill()
    }
  }

  // Matriz: o que fica listado dentro de uma célula (sem célula própria, recuado) ganha a linha do ramo em cotovelo,
  // do pai até ele, para o recuo ler como "filho de", não como desalinhado (pedido do usuário).
  if (table) {
    for (const n of snapshot.nodes) {
      const p = n.parent !== null ? byId.get(n.parent) : undefined
      if (n.cell || !p) continue
      const ex = p.x + Math.max(10, (n.x - p.x) / 2)
      const my = n.y + n.height / 2
      ctx.beginPath()
      ctx.moveTo(ex, p.y + p.height)
      ctx.lineTo(ex, my - 6)
      ctx.quadraticCurveTo(ex, my, ex + 6, my)
      ctx.lineTo(n.x, my)
      ctx.strokeStyle = branch.get(n.id) ?? colors.muted
      ctx.lineWidth = 1.5
      ctx.lineCap = 'round'
      ctx.stroke()
    }
  }

  // Ramos primeiro, para os nós ficarem por cima.
  for (const n of table ? [] : snapshot.nodes) {
    if (n.parent === null) continue
    const p = byId.get(n.parent)
    // Na linha do tempo, cada trecho da linha vai do principal anterior (ou da raiz) até este.
    const i = p ? p.child_ids.indexOf(n.id) : -1
    const prev = p && snapshot.structure === 'linha' && p.parent === null && i > 0 ? byId.get(p.child_ids[i - 1]) : undefined
    if (p) drawBranch(ctx, prev ?? p, n, theme, branch.get(n.id) ?? colors.muted, linkStyle, snapshot.structure, prev !== undefined)
  }

  for (const n of snapshot.nodes) {
    drawNode(ctx, n, {
      colors,
      theme,
      mode,
      branch: branch.get(n.id) ?? colors.muted,
      isSelected: n.id === selected || picked.has(n.id),
      isHovered: n.id === hovered,
      isMatch: matches.has(n.id),
      paintCards: o.paintCards ?? false,
      cards: o.cards,
      tableBg: table ? tableBg.get(n.id) : undefined,
    })
  }

  // Notas soltas: na tela são HTML por cima do canvas; na exportação, o mesmo card repetido aqui.
  if (o.paintCards && o.cards) {
    for (const f of snapshot.notes) {
      const p = o.cards.get(cardKey('f', f.id))
      if (p) replay(ctx, p, f.x, f.y)
    }
  }

  // Botão de recolher/expandir no começo do ramo. Na exportação, só o de quem está recolhido (o número diz o que
  // ficou escondido); o "−" é só para a tela.
  {
    for (const n of snapshot.nodes) {
      if (table || !n.has_children || n.parent === null || (o.paintCards && !n.folded)) continue
      drawKnob(ctx, n, {
        branch: branch.get(n.id) ?? colors.muted,
        colors,
        active: n.id === hovered || n.id === selected || picked.has(n.id),
        down: snapshot.structure === 'organograma',
        style: levelStyle(theme, n.depth),
      })
    }
  }
}

/**
 * Tela de toque (sem "mouse em cima"): o botão de recolher fica sempre à vista. Com mouse, aparece ao passar sobre o
 * tópico ou com ele escolhido, como no XMind (escolha do usuário).
 */
const touch = typeof matchMedia !== 'undefined' && matchMedia('(hover: none)').matches

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
    ctx.beginPath()
    ctx.arc(x, y, KNOB_R + (label.length > 1 ? 1.5 : 0), 0, Math.PI * 2)
    ctx.fillStyle = o.branch
    ctx.fill()
    ctx.strokeStyle = o.colors.stage
    ctx.lineWidth = 2
    ctx.stroke()
    ctx.fillStyle = textOn(o.branch, '#ffffff', '#16201c')
    ctx.fillText(label, x, y + 0.5)
  } else if (o.active || touch) {
    // Aberto: "−" para recolher.
    ctx.beginPath()
    ctx.arc(x, y, KNOB_R, 0, Math.PI * 2)
    ctx.fillStyle = o.colors.surface
    ctx.fill()
    ctx.strokeStyle = o.branch
    ctx.lineWidth = 1.5
    ctx.stroke()
    ctx.lineWidth = 1.8
    ctx.lineCap = 'round'
    ctx.beginPath()
    ctx.moveTo(x - 3.6, y)
    ctx.lineTo(x + 3.6, y)
    ctx.stroke()
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
  if (isTable(snapshot.structure)) return null
  for (let i = snapshot.nodes.length - 1; i >= 0; i--) {
    const n = snapshot.nodes[i]
    if (!n.has_children || n.parent === null) continue
    const k = knobAt(n, levelStyle(theme, n.depth), down)
    if (Math.hypot(wx - k.x, wy - k.y) <= r) return n.id
  }
  return null
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
  /** Linha do tempo: `parent` é o principal anterior, não o pai. */
  alongLine = false,
) {
  ctx.save()
  ctx.strokeStyle = color
  ctx.lineWidth = lineWidth(theme, child.depth)
  ctx.lineCap = 'round'
  ctx.beginPath()
  if (structure === 'organograma') {
    // De baixo do pai até o alto do filho.
    traceBranchDown(ctx, style, parent.x + parent.width / 2, parent.y + parent.height, child.x + child.width / 2, child.y)
  } else if (structure === 'linha' && (alongLine || parent.parent === null)) {
    // Linha do tempo: os tópicos principais em sequência sobre a linha que sai da raiz.
    const mid = parent.y + parent.height / 2
    ctx.moveTo(parent.x + parent.width, mid)
    ctx.lineTo(child.x, child.y + child.height / 2)
  } else if (structure === 'linha' && parent.depth === 1) {
    // Do tópico principal sobe (ou desce) uma haste e entra no subtópico pela esquerda.
    const up = child.y < parent.y
    traceElbow(ctx, parent.x + 12, up ? parent.y : parent.y + parent.height, child.x, anchorY(child, levelStyle(theme, child.depth)))
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
  cards?: Map<string, CardPaint>
  /** Estrutura de tabela: a cor da célula por baixo do tópico (o tópico vira só o texto). */
  tableBg?: string
}

/** O recheio de um nível do tema, já na cor do ramo. */
function fillOf(kind: LevelStyle['fill'], colors: ThemeColors, branch: string, mode: 'light' | 'dark'): string | null {
  return kind === 'accent' ? colors.accent
    : kind === 'branch' ? branch
    : kind === 'soft' ? softFill(branch, colors.surface, mode)
    : kind === 'surface' ? colors.surface
    : null
}

/** As estruturas desenhadas como tabela (células no lugar das linhas dos ramos). */
export const isTable = (structure: string | null | undefined) => structure === 'matriz' || structure === 'tabela'

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

  // Na tabela, o recheio é da célula (já pintada em `draw`); o tópico é só o texto sobre ela.
  const fill = o.tableBg !== undefined ? null : fillOf(style.fill, colors, branch, mode)

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

  // PRÉVIA DE OPÇÕES (provisório): `?chip=a|b` dá caixa própria a cada tópico dentro da célula.
  const chip = o.tableBg !== undefined && n.depth >= 2 ? (typeof location !== 'undefined' ? new URLSearchParams(location.search).get('chip') : null) : null
  let chipFill: string | null = null
  if (chip === 'a') {
    roundRect(ctx, x, y, w, h, 10)
    ctx.fillStyle = colors.surface
    ctx.fill()
    ctx.strokeStyle = colors.cardEdge
    ctx.lineWidth = 1
    ctx.stroke()
    chipFill = colors.surface
  } else if (chip === 'b') {
    roundRect(ctx, x, y, w, h, 8)
    ctx.globalAlpha = 0.6
    ctx.fillStyle = colors.surface
    ctx.fill()
    ctx.globalAlpha = 1
    ctx.save()
    roundRect(ctx, x, y, w, h, 8)
    ctx.clip()
    ctx.fillStyle = branch
    ctx.fillRect(x, y, 4, h)
    ctx.restore()
  }

  // Contorno do estilo.
  if (o.tableBg !== undefined) {
    // sem contorno: a célula já é a caixa
  } else if (style.border !== 'none') {
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
  if (style.underline && o.tableBg === undefined) {
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
  const ink =
    chipFill ? textOn(chipFill, colors.fg, colors.onAccent)
    : o.tableBg !== undefined ? textOn(o.tableBg, colors.fg, colors.onAccent)
    : style.text === 'branch' ? branch
    : textOn(fill ?? colors.stage, colors.fg, colors.onAccent)
  ctx.fillStyle = ink
  ctx.textAlign = 'left'
  ctx.textBaseline = 'middle'
  // As mesmas linhas da medição (`wrapText` na largura da caixa), alinhadas à esquerda como no card (pedido do
  // usuário); o bloco de texto fica no meio da caixa, então uma linha só continua centrada.
  const lines = wrapText(n.text, w - style.padX + 0.5, style)
  const widest = Math.max(...lines.map((l) => ctx.measureText(l).width))
  const left = x + (w - widest) / 2
  const lineH = style.size * 1.4
  const top = y + h / 2 - (lines.length * lineH) / 2 + lineH / 2 - (style.underline && o.tableBg === undefined ? 2 : 0)
  lines.forEach((line, i) => ctx.fillText(line, left, top + i * lineH))

  ctx.restore()
}

/** Um nó com nota. Seleção e recolhido sempre; o card em si só quando `paintCards` (exportação). */
function drawCard(ctx: CanvasRenderingContext2D, n: FlatNode, o: NodeOpts) {
  const { colors } = o
  const { x, y, width: w, height: h } = n
  ctx.save()
  const painted = o.paintCards ? o.cards?.get(cardKey('n', n.id)) : undefined
  if (painted) replay(ctx, painted, x, y)
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

/** Um contorno de cantos arredondados, um raio por canto. */
function radiiPath(ctx: CanvasRenderingContext2D, x: number, y: number, w: number, h: number, r: Radii) {
  const k = (v: number) => Math.min(v, w / 2, h / 2)
  const [tl, tr, br, bl] = r.map(k)
  ctx.beginPath()
  ctx.moveTo(x + tl, y)
  ctx.lineTo(x + w - tr, y)
  if (tr) ctx.arcTo(x + w, y, x + w, y + tr, tr)
  ctx.lineTo(x + w, y + h - br)
  if (br) ctx.arcTo(x + w, y + h, x + w - br, y + h, br)
  ctx.lineTo(x + bl, y + h)
  if (bl) ctx.arcTo(x, y + h, x, y + h - bl, bl)
  ctx.lineTo(x, y + tl)
  if (tl) ctx.arcTo(x, y, x + tl, y, tl)
  ctx.closePath()
}

/** Repete um card lido da tela (`domcard.ts`) com o canto em (ox, oy). */
function replay(ctx: CanvasRenderingContext2D, p: CardPaint, ox: number, oy: number) {
  ctx.save()
  ctx.textAlign = 'left'
  ctx.textBaseline = 'alphabetic'
  for (const op of p.ops) {
    ctx.globalAlpha = op.alpha
    if (op.t === 'box') {
      // A borda do CSS fica por dentro da caixa.
      const i = op.stroke ? op.sw / 2 : 0
      radiiPath(ctx, ox + op.x + i, oy + op.y + i, op.w - i * 2, op.h - i * 2, op.r)
      if (op.fill) {
        ctx.fillStyle = op.fill
        ctx.fill()
      }
      if (op.stroke) {
        ctx.strokeStyle = op.stroke
        ctx.lineWidth = op.sw
        ctx.stroke()
      }
    } else if (op.t === 'text') {
      ctx.font = op.font
      ctx.fillStyle = op.color
      ctx.fillText(op.text, ox + op.x, oy + op.y)
      if (op.strike != null) {
        ctx.beginPath()
        ctx.moveTo(ox + op.x, oy + op.strike)
        ctx.lineTo(ox + op.x + op.width, oy + op.strike)
        ctx.strokeStyle = op.color
        ctx.lineWidth = 1
        ctx.stroke()
      }
    } else if (op.t === 'img') {
      ctx.save()
      if (op.clip) {
        radiiPath(ctx, ox + op.clip.x, oy + op.clip.y, op.clip.w, op.clip.h, op.clip.r)
        ctx.clip()
      }
      ctx.drawImage(op.img, ox + op.x, oy + op.y, op.w, op.h)
      ctx.restore()
    } else {
      const svg = ctx as unknown as { drawSvgMarkup?: (m: string, x: number, y: number, w: number, h: number) => void }
      if (svg.drawSvgMarkup) svg.drawSvgMarkup(op.markup, ox + op.x, oy + op.y, op.w, op.h)
      else if (op.img.complete && op.img.naturalWidth) ctx.drawImage(op.img, ox + op.x, oy + op.y, op.w, op.h)
    }
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
  // Na tabela, a célula inteira é do tópico.
  for (let i = snapshot.nodes.length - 1; i >= 0; i--) {
    const c = snapshot.nodes[i].cell
    if (c && wx >= c[0] && wx <= c[0] + c[2] && wy >= c[1] && wy <= c[1] + c[3]) return snapshot.nodes[i].id
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

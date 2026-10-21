// Temas de mapa, como os do XMind: um "esqueleto" (forma, recheio e texto de cada nível) e uma paleta que pinta cada
// ramo principal de uma cor, herdada pelos filhos. O tema é do mapa (fica no arquivo, `nodo-style`); a paleta de
// interface (Configurações → Aparência) continua decidindo fundo, superfície e destaque.
//
// Os nomes têm de bater com `nodo_core::MAP_STYLES`: o núcleo recusa qualquer outro.

import type { FlatNode } from './api'
import { contrast, mix } from './palettes'

export type MapThemeId = 'classico' | 'contorno' | 'sublinhado' | 'vibrante' | 'pilulas' | 'mono'

/** Como um nível do mapa é desenhado. */
export interface LevelStyle {
  /** Recheio: cor do ramo (forte ou suave), superfície, destaque do tema, ou nenhum (só texto). */
  fill: 'branch' | 'soft' | 'surface' | 'accent' | 'none'
  /** Contorno: cor do ramo, o contorno discreto dos cards, ou nenhum. */
  border: 'branch' | 'edge' | 'none'
  /** Texto sobre o recheio: escolhido por contraste ('auto') ou na cor do ramo. */
  text: 'auto' | 'branch'
  /** Linha sob o texto na cor do ramo, onde o ramo se liga (estilo sublinhado do XMind). */
  underline: boolean
  /** Raio dos cantos, em fração da altura (0.5 = pílula). */
  round: number
  size: number
  weight: number
  padX: number
  padY: number
  minW: number
  minH: number
}

export interface MapTheme {
  id: MapThemeId
  label: string
  hint: string
  root: LevelStyle
  main: LevelStyle
  sub: LevelStyle
  /** De onde vêm as cores dos ramos. */
  palette: 'classic' | 'rainbow' | 'pastel' | 'mono'
  /** Espessura da linha que sai da raiz; as de baixo afinam até `lineMin`. */
  lineRoot: number
  lineMin: number
}

const base = { underline: false, padX: 36, padY: 22, minW: 84, minH: 44 }

export const MAP_THEMES: Record<MapThemeId, MapTheme> = {
  classico: {
    id: 'classico',
    label: 'Clássico',
    hint: 'Cartões em cores suaves',
    root: { ...base, fill: 'accent', border: 'none', text: 'auto', round: 0.34, size: 15, weight: 700, padX: 40, padY: 26, minH: 48 },
    main: { ...base, fill: 'branch', border: 'none', text: 'auto', round: 0.34, size: 13.5, weight: 600 },
    sub: { ...base, fill: 'branch', border: 'none', text: 'auto', round: 0.34, size: 13, weight: 500, minH: 38, padY: 18 },
    palette: 'classic',
    lineRoot: 2.4,
    lineMin: 2.4,
  },
  contorno: {
    id: 'contorno',
    label: 'Contorno',
    hint: 'Fundo claro, borda na cor do ramo',
    root: { ...base, fill: 'surface', border: 'branch', text: 'auto', round: 0.3, size: 16, weight: 700, padX: 44, padY: 28, minH: 52 },
    main: { ...base, fill: 'surface', border: 'branch', text: 'auto', round: 0.3, size: 14, weight: 600 },
    sub: { ...base, fill: 'surface', border: 'edge', text: 'auto', round: 0.3, size: 13, weight: 500, minH: 38, padY: 18 },
    palette: 'rainbow',
    lineRoot: 2.6,
    lineMin: 1.6,
  },
  sublinhado: {
    id: 'sublinhado',
    label: 'Sublinhado',
    hint: 'Texto sobre a linha, como no XMind',
    root: { ...base, fill: 'accent', border: 'none', text: 'auto', round: 0.5, size: 16, weight: 700, padX: 44, padY: 28, minH: 52 },
    main: { ...base, fill: 'branch', border: 'none', text: 'auto', round: 0.3, size: 14, weight: 600 },
    sub: { ...base, fill: 'none', border: 'none', text: 'auto', underline: true, round: 0, size: 13, weight: 500, padX: 20, padY: 14, minW: 40, minH: 30 },
    palette: 'rainbow',
    lineRoot: 3,
    lineMin: 1.6,
  },
  vibrante: {
    id: 'vibrante',
    label: 'Vibrante',
    hint: 'Cores fortes, linhas grossas',
    root: { ...base, fill: 'accent', border: 'none', text: 'auto', round: 0.3, size: 17, weight: 800, padX: 48, padY: 30, minH: 56 },
    main: { ...base, fill: 'branch', border: 'none', text: 'auto', round: 0.3, size: 14.5, weight: 700 },
    sub: { ...base, fill: 'branch', border: 'none', text: 'auto', round: 0.3, size: 13, weight: 600, minH: 38, padY: 18 },
    palette: 'rainbow',
    lineRoot: 4.5,
    lineMin: 2.2,
  },
  pilulas: {
    id: 'pilulas',
    label: 'Pílulas',
    hint: 'Tudo arredondado',
    root: { ...base, fill: 'accent', border: 'none', text: 'auto', round: 0.5, size: 15, weight: 700, padX: 44, padY: 26, minH: 48 },
    main: { ...base, fill: 'soft', border: 'branch', text: 'auto', round: 0.5, size: 13.5, weight: 600, padX: 40 },
    sub: { ...base, fill: 'soft', border: 'none', text: 'auto', round: 0.5, size: 13, weight: 500, padX: 36, minH: 36, padY: 16 },
    palette: 'rainbow',
    lineRoot: 2.4,
    lineMin: 1.8,
  },
  mono: {
    id: 'mono',
    label: 'Monocromático',
    hint: 'Cinzas, só a raiz em destaque',
    root: { ...base, fill: 'accent', border: 'none', text: 'auto', round: 0.3, size: 15, weight: 700, padX: 40, padY: 26, minH: 48 },
    main: { ...base, fill: 'surface', border: 'branch', text: 'auto', round: 0.3, size: 13.5, weight: 600 },
    sub: { ...base, fill: 'none', border: 'none', text: 'auto', underline: true, round: 0, size: 13, weight: 500, padX: 20, padY: 14, minW: 40, minH: 30 },
    palette: 'mono',
    lineRoot: 2,
    lineMin: 1.4,
  },
}

export const MAP_THEME_IDS = Object.keys(MAP_THEMES) as MapThemeId[]

/** O tema de um mapa; sem escolha (ou um nome que esta versão não conhece), o Clássico. */
export function mapTheme(style: string | null | undefined): MapTheme {
  return (style && MAP_THEMES[style as MapThemeId]) || MAP_THEMES.classico
}

/** O estilo do nível de um nó. */
export function levelStyle(theme: MapTheme, depth: number): LevelStyle {
  return depth === 0 ? theme.root : depth === 1 ? theme.main : theme.sub
}

/** Espessura da linha que chega a um nó nesta profundidade. */
export function lineWidth(theme: MapTheme, depth: number): number {
  const t = Math.min(1, Math.max(0, (depth - 1) / 2))
  return theme.lineRoot + (theme.lineMin - theme.lineRoot) * t
}

// As cores médias do mapa de antes dos temas (o Clássico): recheio com texto escuro.
const CLASSIC = ['#6eb0f7', '#b28ee6', '#7fbf7f', '#e8c547', '#f0a36b', '#e5818d', '#5cc8c8', '#c9a27e']
// Cores fortes que funcionam no claro e no escuro (linhas e recheios 'branch').
const RAINBOW = ['#3f7bf5', '#9b5de5', '#22a868', '#f0a020', '#ef5a5f', '#0aa5c0', '#e0559c', '#7a8a1f']
const MONO_LIGHT = ['#5f6b75']
const MONO_DARK = ['#9aa5ae']

/**
 * A cor de cada ramo. Ramos principais recebem a próxima cor da paleta, na ordem; os filhos herdam a do pai.
 * Uma cor escolhida no nó (do arquivo ou do painel) vale para ele e para os de baixo.
 * `pastel` é a lista de cores de nó do tema de interface (--n-*), já resolvida.
 */
export function branchColors(
  nodes: FlatNode[],
  theme: MapTheme,
  mode: 'light' | 'dark',
  pastel: string[],
  resolve: (c: string) => string,
): Map<number, string> {
  const palette =
    theme.palette === 'classic' ? CLASSIC
    : theme.palette === 'rainbow' ? RAINBOW
    : theme.palette === 'mono' ? (mode === 'dark' ? MONO_DARK : MONO_LIGHT)
    : pastel
  const out = new Map<number, string>()
  let next = 0
  // Os nós vêm em pré-ordem: o pai sempre antes dos filhos.
  for (const n of nodes) {
    if (n.color) out.set(n.id, resolve(n.color))
    else if (n.parent === null) continue
    else if (n.depth === 1) out.set(n.id, palette[next++ % palette.length])
    else {
      const inherited = out.get(n.parent)
      if (inherited) out.set(n.id, inherited)
    }
    if (n.depth === 1 && n.color) next++
  }
  return out
}

/** Recheio suave a partir da cor do ramo: misturada à superfície, para o texto escuro (ou claro) continuar legível. */
export function softFill(color: string, surface: string, mode: 'light' | 'dark'): string {
  // `mix` mede quanto entra da primeira cor: um quarto do ramo sobre a superfície (um pouco mais no escuro).
  return mix(color, surface, mode === 'dark' ? 0.4 : 0.26)
}

/** Texto legível sobre um recheio: o do tema, ou branco/escuro, o que tiver mais contraste. */
export function textOn(fill: string, fg: string, onAccent: string): string {
  const candidates = [fg, onAccent, '#ffffff', '#16201c']
  return candidates.reduce((best, c) => (contrast(c, fill) > contrast(best, fill) ? c : best), candidates[0])
}

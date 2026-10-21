// Temas de cor: moram no Atrio (`@wylho/atrio`), os mesmos do Ideario. A paleta-base, que aqui se chamava Nodo, é a
// Atrio; quem tinha escolhido continua nela (`normalizePaletteId`). Aqui fica só o que é do mapa: o tema Sistema
// montado com o destaque que o núcleo lê, e os nomes que o mapa usa para as cores de nota (cor do ramo).
import {
  DESKTOPS,
  fitAccent,
  NOTE_COLOR_TOKENS,
  PALETTES,
  resolveNoteColor,
  type Colors,
  type DesktopId,
  type Mode,
  type PaletteId,
} from '@wylho/atrio'

export {
  contrast,
  cssVars,
  DESKTOPS,
  fitAccent,
  mix,
  normalizePaletteId,
  PALETTES,
  type Colors,
  type DesktopId,
  type Mode,
  type PaletteId,
} from '@wylho/atrio'

/** Cores do tema Sistema: paleta da área de trabalho + destaque do sistema. */
export function systemColors(desktopId: DesktopId, mode: Mode, systemAccent: string | null): Colors {
  const d = DESKTOPS[desktopId] || DESKTOPS.gnome
  const base = { ...d[mode] }
  base.accent = fitAccent(systemAccent || d.accent, mode, base.surface)
  return base
}

export function paletteColors(id: PaletteId, mode: Mode, desktopId: DesktopId, systemAccent: string | null): Colors {
  return id === 'system' ? systemColors(desktopId, mode, systemAccent) : PALETTES[id][mode]
}

/** Cores de ramo = cores de nota do Atrio, na mesma ordem. */
export const NODE_COLOR_TOKENS = NOTE_COLOR_TOKENS

/**
 * Resolve um token de cor de nó (`--n-sky`) para o hex que o arquivo guarda no tema ativo. É a única ponte entre os
 * tokens da interface e a cor concreta, para nenhuma outra parte gravar um token no lugar de uma cor.
 */
export const resolveNodeColor: (token: string, colors: Colors) => string = resolveNoteColor

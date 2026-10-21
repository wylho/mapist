// Tema (paleta de cores) e aparência (claro, escuro ou automático), escolhidos
// pelo usuário. Portado de ideario/src/lib/theme.svelte.ts.
//
// Aplicar é só gravar ~25 variáveis CSS no <html>: nenhum componente muda.

import {
  cssVars,
  DESKTOPS,
  PALETTES,
  paletteColors,
  type Colors,
  type DesktopId,
  type Mode,
  type PaletteId,
} from './palettes'

/** Aparência: claro, escuro ou acompanhar o sistema. */
export type ThemePref = 'system' | 'light' | 'dark'

/** O que o núcleo Rust leu da área de trabalho, quando disponível. */
export interface SystemTheme {
  desktop?: DesktopId | null
  accent?: string | null
}

declare global {
  interface Window {
    __MAPIST_SYSTEM_THEME__?: SystemTheme
  }
}

const MODE_KEY = 'mapist.theme'
const PALETTE_KEY = 'mapist.palette'
const SIM_KEY = 'mapist.simDesktop'

const load = (key: string): string | null => {
  try {
    return localStorage.getItem(key)
  } catch {
    return null
  }
}

const store = (key: string, v: string) => {
  try {
    localStorage.setItem(key, v)
  } catch {
    // sem armazenamento: vale só nesta sessão
  }
}

const readMode = (): ThemePref => {
  const v = load(MODE_KEY)
  return v === 'light' || v === 'dark' ? v : 'system'
}

const readPalette = (): PaletteId => {
  const v = load(PALETTE_KEY)
  // 'ideario' era o nome desta paleta até a 0.1; quem escolheu continua nela.
  if (v === 'ideario') return 'mapist'
  return v === 'system' || (v && v in PALETTES) ? (v as PaletteId) : 'mapist'
}

/** No navegador (prévia), adivinha a área de trabalho pelo user agent. */
function guessDesktop(): DesktopId {
  const saved = load(SIM_KEY)
  if (saved && saved in DESKTOPS) return saved as DesktopId
  const ua = navigator.userAgent
  return /Mac/.test(ua) ? 'macos' : /Windows/.test(ua) ? 'windows' : 'gnome'
}

const darkQuery = typeof matchMedia === 'function' ? matchMedia('(prefers-color-scheme: dark)') : null

class Theme {
  /** Aparência. */
  pref = $state<ThemePref>(readMode())
  /** Paleta de cores. */
  palette = $state<PaletteId>(readPalette())
  /** Qual área de trabalho o tema Sistema imita. */
  simDesktop = $state<DesktopId>(guessDesktop())
  /** Destaque lido do sistema, quando o host fornece. */
  systemAccent = $state<string | null>(null)
  private osDark = $state(darkQuery?.matches ?? false)

  get mode(): Mode {
    return this.pref === 'system' ? (this.osDark ? 'dark' : 'light') : this.pref
  }

  /** As cores resolvidas para a paleta e o modo atuais. */
  get colors(): Colors {
    return paletteColors(this.palette, this.mode, this.simDesktop, this.systemAccent)
  }

  /** O rótulo do tema ativo, para a barra. */
  get label(): string {
    return this.palette === 'system'
      ? DESKTOPS[this.simDesktop].label
      : PALETTES[this.palette].label
  }

  constructor() {
    darkQuery?.addEventListener('change', (e) => {
      this.osDark = e.matches
      this.apply()
    })
    this.apply()
  }

  set(pref: ThemePref) {
    this.pref = pref
    store(MODE_KEY, pref)
    this.apply()
  }

  setPalette(id: PaletteId) {
    this.palette = id
    store(PALETTE_KEY, id)
    this.apply()
  }

  setSimDesktop(id: DesktopId) {
    this.simDesktop = id
    store(SIM_KEY, id)
    this.apply()
  }

  /** Alterna claro/escuro, saindo do modo automático. */
  toggleMode() {
    this.set(this.mode === 'dark' ? 'light' : 'dark')
  }

  /** Grava as variáveis CSS no <html>. */
  apply() {
    if (typeof document === 'undefined') return
    const root = document.documentElement
    const vars = cssVars(this.colors, this.mode)
    for (const [k, v] of Object.entries(vars)) root.style.setProperty(k, v)
    root.style.colorScheme = this.mode
    // Mantém o seletor [data-theme] coerente para o CSS que ainda o usa.
    if (this.pref === 'system') root.removeAttribute('data-theme')
    else root.setAttribute('data-theme', this.pref)
    // No app, a barra de título (macOS, Windows) e o fundo da janela acompanham, como no Ideario.
    if ('__TAURI_INTERNALS__' in window) {
      const pref = this.pref
      const bg = this.colors.bg
      import('@tauri-apps/api/window')
        .then(({ getCurrentWindow }) => {
          const w = getCurrentWindow()
          return Promise.all([w.setTheme(pref === 'system' ? null : pref), w.setBackgroundColor(bg)])
        })
        .catch(() => {})
    }
  }
}

export const theme = new Theme()

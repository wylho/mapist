// Menus de contexto do mapa (clique direito, tecla de menu, ⇧F10), no formato dos menus do Ideario (MenuEntry).
// Cada item repete uma ação que já existe por atalho ou na barra, com o atalho ao lado para quem quiser aprender.
import {
  ClipboardPaste,
  Copy,
  ArrowDown,
  ArrowUp,
  ChevronsDownUp,
  ChevronsUpDown,
  CornerDownRight,
  Focus,
  Maximize,
  Palette,
  PanelLeft,
  PanelRight,
  Pencil,
  Plus,
  Redo2,
  Search,
  Share,
  Spline,
  StickyNote,
  Trash2,
  Undo2,
  WandSparkles,
  Network,
  ZoomIn,
} from '@lucide/svelte'
import { SPACINGS, STRUCTURES, spacingOf, structureOf } from './structures'
import type { MenuEntry } from '@wylho/atrio'
import { SEP } from '@wylho/atrio'
import { doc } from './document.svelte'
import { ui } from './ui.svelte'
import { prefs } from './prefs.svelte'
import { theme } from './theme.svelte'
import { notes } from './notes.svelte'
import { readClipboardCard, writeCard } from '@wylho/atrio'
import { resolveNodeColor } from './palettes'

const mac = typeof navigator !== 'undefined' && /Mac/.test(navigator.platform)
const mod = mac ? '⌘' : 'Ctrl+'
const alt = mac ? '⌥' : 'Alt+'

/** O que o menu do fundo pede ao canvas (enquadrar e zoom vivem lá). */
export interface ViewActions {
  fit: () => void
  fitSelection: () => void
  actualSize: () => void
  /** Nota solta no ponto da janela onde o menu abriu. */
  addNoteAt: (x: number, y: number) => void
}

const COLORS: [string, string][] = [
  ['--n-sand', 'Areia'],
  ['--n-sage', 'Sálvia'],
  ['--n-sky', 'Céu'],
  ['--n-rose', 'Rosa'],
  ['--n-lilac', 'Lilás'],
  ['--n-butter', 'Manteiga'],
]

function tokenColor(token: string): string {
  return getComputedStyle(document.documentElement).getPropertyValue(token).trim() || 'transparent'
}

/** Copiar o nó como card do Atrio e colar um card (do Ideario ou de outro nó) como filho. */
function clipItems(withCopy: boolean): MenuEntry[] {
  const paste: MenuEntry = {
    label: 'Colar card',
    icon: ClipboardPaste,
    hint: `${mod}V`,
    onSelect: async () => {
      const card = await readClipboardCard()
      if (card) await doc.pasteCard(card)
      else ui.say('Nenhum card copiado. No Ideario, copie a nota com ⌘C')
    },
  }
  if (!withCopy) return [paste]
  return [
    {
      label: 'Copiar como card',
      icon: Copy,
      hint: `${mod}C`,
      onSelect: async () => {
        const card = doc.selectedCard()
        if (!card) return
        try {
          await writeCard(card)
          ui.say('Copiado como card')
        } catch {
          ui.say('Não foi possível copiar')
        }
      },
    },
    paste,
  ]
}

/** Menu de um nó. Seleciona o nó antes de abrir, para que as ações (e os atalhos) valham para ele. */
export function nodeMenu(id: number, view: ViewActions): MenuEntry[] {
  const n = doc.node(id)
  if (!n) return []
  const isRoot = n.parent === null
  const siblings = isRoot ? [] : (doc.node(n.parent)?.child_ids ?? [])
  const sameSide = (sid: number) => doc.node(n.parent)?.parent !== null || doc.node(sid)?.side === n.side
  const pos = siblings.indexOf(id)
  const canUp = siblings.slice(0, pos).some(sameSide)
  const canDown = siblings.slice(pos + 1).some(sameSide)

  const items: MenuEntry[] = [
    { label: 'Editar texto', icon: Pencil, hint: 'Espaço', onSelect: () => doc.startEdit() },
    { label: 'Adicionar filho', icon: Plus, hint: 'Tab', onSelect: () => void doc.addChild() },
  ]
  items.splice(
    1,
    0,
    n.note
      ? { label: notes.isOpen(id) ? 'Esconder a nota' : 'Mostrar a nota', icon: StickyNote, onSelect: () => notes.toggle(id) }
      : { label: 'Adicionar nota', icon: StickyNote, onSelect: () => notes.show(id, true) },
  )
  if (!isRoot) items.push({ label: 'Adicionar irmão', icon: CornerDownRight, hint: 'Enter', onSelect: () => void doc.addSibling() })
  items.push(SEP)
  items.push({
    label: 'Cor do ramo',
    icon: Palette,
    sub: [
      { label: 'Sem cor', checked: !n.color, onSelect: () => void doc.setColor(id, null) },
      SEP,
      ...COLORS.map(([token, name]): MenuEntry => {
        const value = resolveNodeColor(token, theme.colors)
        return { label: name, swatch: tokenColor(token), checked: n.color === value, onSelect: () => void doc.setColor(id, value) }
      }),
    ],
  })
  if (n.depth === 1) {
    items.push({
      label: 'Lado do ramo',
      icon: n.side === 'left' ? PanelLeft : PanelRight,
      sub: [
        { label: 'Esquerda', icon: PanelLeft, checked: n.side === 'left', onSelect: () => void doc.setSide(id, 'left') },
        { label: 'Direita', icon: PanelRight, checked: n.side !== 'left', onSelect: () => void doc.setSide(id, 'right') },
      ],
    })
  }
  if (n.has_children) {
    items.push({
      label: n.folded ? 'Expandir ramo' : 'Recolher ramo',
      icon: n.folded ? ChevronsUpDown : ChevronsDownUp,
      hint: `${mod}/`,
      onSelect: () => void doc.toggleFold(),
    })
  }
  if (!isRoot) {
    items.push(
      { label: 'Subir', icon: ArrowUp, hint: `${alt}↑`, disabled: !canUp, onSelect: () => void doc.reorder(-1) },
      { label: 'Descer', icon: ArrowDown, hint: `${alt}↓`, disabled: !canDown, onSelect: () => void doc.reorder(1) },
    )
  }
  items.push({ label: 'Enquadrar este ramo', icon: Focus, hint: '⇧2', onSelect: view.fitSelection })
  items.push(SEP, ...clipItems(true))
  if (!isRoot) {
    items.push(SEP, { label: 'Excluir', icon: Trash2, hint: 'Delete', danger: true, onSelect: () => void doc.remove() })
  }
  return items
}

/** Menu do fundo do mapa: ver, zoom, aparência e histórico. */
export function backgroundMenu(view: ViewActions, at: { x: number; y: number }): MenuEntry[] {
  return [
    { label: 'Nova nota aqui', icon: StickyNote, onSelect: () => view.addNoteAt(at.x, at.y) },
    ...clipItems(false),
    SEP,
    { label: 'Ver o mapa inteiro', icon: Maximize, hint: '⇧1', onSelect: view.fit },
    { label: 'Tamanho real', icon: ZoomIn, hint: `${mod}0`, onSelect: view.actualSize },
    SEP,
    {
      label: 'Estrutura',
      icon: Network,
      sub: [
        ...STRUCTURES.map((st): MenuEntry => ({
          label: st.label,
          hint: st.hint,
          checked: structureOf(doc.snapshot.structure) === st.id,
          onSelect: () => void doc.setStructure(st.id),
        })),
        SEP,
        ...SPACINGS.map((sp): MenuEntry => ({
          label: `Espaçamento ${sp.label.toLowerCase()}`,
          checked: spacingOf(doc.snapshot.spacing) === sp.id,
          onSelect: () => void doc.setSpacing(sp.id),
        })),
      ],
    },
    { label: 'Arrumar posições', icon: WandSparkles, onSelect: () => void doc.relayout() },
    { label: 'Tema do mapa…', icon: Palette, onSelect: () => (ui.themeOpen = true) },
    { label: 'Exportar…', icon: Share, hint: `${mod}⇧E`, onSelect: () => (ui.exportOpen = true) },
    {
      label: 'Linhas',
      icon: Spline,
      sub: [
        { label: 'Curvas', checked: prefs.linkStyle === 'curved', onSelect: () => prefs.setLinkStyle('curved') },
        { label: 'Em ângulo', checked: prefs.linkStyle === 'orthogonal', onSelect: () => prefs.setLinkStyle('orthogonal') },
      ],
    },
    { label: 'Buscar no mapa', icon: Search, hint: `${mod}F`, onSelect: () => {
      ui.searchOpen = true
      requestAnimationFrame(() => document.getElementById('busca')?.focus())
    } },
    SEP,
    { label: 'Desfazer', icon: Undo2, hint: `${mod}Z`, disabled: !doc.snapshot.can_undo, onSelect: () => void doc.undo() },
    { label: 'Refazer', icon: Redo2, hint: `${mod}⇧Z`, disabled: !doc.snapshot.can_redo, onSelect: () => void doc.redo() },
  ]
}

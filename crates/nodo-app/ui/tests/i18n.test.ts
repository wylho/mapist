// Idiomas: o dicionário inglês cobre todo texto da interface, os rótulos traduzem na hora de mostrar e o mapa de
// exemplo da prévia segue o idioma.

import { readdirSync, readFileSync, statSync } from 'node:fs'
import { join } from 'node:path'
import { afterEach, describe, expect, it } from 'vitest'
import { addMessages, setLocale, t, tn } from '@wylho/atrio'
import { EN } from '../src/lib/i18n/en'
import { SPACINGS, STRUCTURES } from '../src/lib/structures'
import { MAP_THEMES } from '../src/lib/maptheme'
import { FORMATS } from '../src/lib/exporter'

addMessages('en', EN)
afterEach(() => setLocale('pt-BR'))

/** Todo `t('…')`/`tn(n, '…', '…')` escrito nos arquivos da interface. */
function keysInSource(dir = join(__dirname, '../src')): string[] {
  const out: string[] = []
  for (const name of readdirSync(dir)) {
    const path = join(dir, name)
    if (statSync(path).isDirectory()) {
      if (name !== 'i18n') out.push(...keysInSource(path))
      continue
    }
    if (!/\.(ts|svelte)$/.test(name)) continue
    const s = readFileSync(path, 'utf8')
    for (const m of s.matchAll(/\b(?:t|tr)\('((?:[^'\\]|\\.)*)'/g)) out.push(m[1])
    for (const m of s.matchAll(/\btn\([^,]+,\s*'((?:[^'\\]|\\.)*)',\s*'((?:[^'\\]|\\.)*)'/g)) out.push(m[1], m[2])
  }
  return out
}

describe('dicionário inglês', () => {
  it('tem todo texto da interface', () => {
    const keys = keysInSource()
    expect(keys.length).toBeGreaterThan(150)
    expect(keys.filter((k) => !(k in EN))).toEqual([])
  })

  it('tem os rótulos das listas (estruturas, espaçamentos, temas, formatos)', () => {
    const labels = [
      ...STRUCTURES.flatMap((s) => [s.label, s.hint]),
      ...SPACINGS.map((s) => s.label),
      ...Object.values(MAP_THEMES).flatMap((m) => [m.label, m.hint]),
      ...FORMATS.flatMap((f) => [f.label, f.hint]),
    ]
    expect(labels.filter((k) => !(k in EN))).toEqual([])
  })

  it('mantém os marcadores de cada tradução', () => {
    const vars = (s: string) => [...s.matchAll(/\{(\w+)\}/g)].map((m) => m[1]).sort()
    for (const [pt, en] of Object.entries(EN)) expect(vars(en), pt).toEqual(vars(pt))
  })
})

describe('rótulos', () => {
  it('saem no idioma escolhido, sem mudar o id gravado no arquivo', () => {
    const matrix = STRUCTURES.find((s) => s.id === 'matriz')!
    expect(t(matrix.label)).toBe('Matriz')
    setLocale('en')
    expect(t(matrix.label)).toBe('Matrix')
    expect(t(STRUCTURES.find((s) => s.id === 'logico')!.label)).toBe('Logic chart')
    expect(t('Nova nota aqui')).toBe('New note here')
    expect(t('Exportado: {nome}', { nome: 'mapa.png' })).toBe('Exported: mapa.png')
    expect(tn(2, '{n} mapa tem alterações que não foram salvas.', '{n} mapas têm alterações que não foram salvas.')).toBe(
      '2 maps have unsaved changes.',
    )
    expect(matrix.id).toBe('matriz')
  })
})

describe('mapa de exemplo da prévia', () => {
  it('sai em inglês com a interface em inglês, com a mesma estrutura do português', async () => {
    const { seedDemo, previewBackend } = await import('../src/lib/preview')
    seedDemo()
    const pt = previewBackend.get_snapshot()
    expect(pt.nodes[0].text).toBe('Lançamento do app')

    setLocale('en')
    seedDemo()
    const en = previewBackend.get_snapshot()
    const root = en.nodes.find((n) => n.parent === null)!
    expect(root.text).toBe('App launch')
    expect(en.tabs[0].path).toBe('example.nodo')
    expect(en.notes.map((n) => n.title)).toEqual(['Loose ideas'])
    expect(en.nodes.find((n) => n.text === 'Build')?.card.emoji).toBe('🛠️')
    expect(en.nodes.find((n) => n.text === 'Beta')?.note).toContain('Invite')
    // A mesma forma: tantos tópicos, os mesmos com nota, o mesmo ramo recolhido.
    const shape = (s: typeof en) => s.nodes.map((n) => [n.depth, n.side, !!n.note, n.folded, n.child_ids.length])
    expect(shape(en)).toEqual(shape(pt))
    expect(en.nodes.some((n) => /[ãçéíóú]/i.test(n.text))).toBe(false)
  })
})

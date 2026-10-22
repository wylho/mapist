// Os últimos mapas abertos ou salvos, para a tela inicial (pedido do usuário: ao abrir o app, os recentes e os
// botões Novo e Abrir no meio do mapa). Ficam no aparelho (`localStorage`, como as outras preferências, D9); o
// documento em si continua só no arquivo.
import { dayDiff, fmtReminder, fmtShortDate } from '@wylho/atrio'

export type Recent = { path: string; at: number }

/** Quantos a tela inicial mostra (escolha do usuário). */
export const MAX_RECENTS = 8
const KEY = 'nodo.recents'

/** O mapa vai para o topo, sem repetir, e a lista não passa de `max`. */
export function pushRecent(list: Recent[], path: string, at: number, max = MAX_RECENTS): Recent[] {
  return [{ path, at }, ...list.filter((r) => r.path !== path)].slice(0, max)
}

/** Nome que a tela mostra: o do arquivo, sem o `.nodo`. */
export function recentName(path: string): string {
  return (path.split(/[\\/]/).pop() ?? path).replace(/\.nodo$/i, '')
}

/** A pasta do arquivo, com a pasta pessoal abreviada como `~` (macOS, Linux e Windows). */
export function recentFolder(path: string): string {
  const dir = path.replace(/[\\/][^\\/]*$/, '')
  return dir.replace(/^(\/Users\/[^/]+|\/home\/[^/]+|[A-Za-z]:\\Users\\[^\\]+)(?=[\\/]|$)/, '~') || dir
}

/** "Hoje, 14:20", "Ontem, 09:05" e, antes disso, só o dia ("12 de out."). */
export function recentWhen(at: number, now = Date.now()): string {
  return dayDiff(at, now) >= -1 ? fmtReminder(at) : fmtShortDate(at)
}

function load(): Recent[] {
  try {
    const raw = JSON.parse(localStorage.getItem(KEY) ?? '[]')
    return Array.isArray(raw)
      ? raw.filter((r) => typeof r?.path === 'string' && typeof r?.at === 'number').slice(0, MAX_RECENTS)
      : []
  } catch {
    return []
  }
}

class Recents {
  list = $state<Recent[]>(load())

  /** Um `.nodo` aberto ou salvo. Importações (Markdown, OPML, `.minder`) não entram: o mapa não está naquele arquivo. */
  add(path: string) {
    if (!/\.nodo$/i.test(path)) return
    this.list = pushRecent(this.list, path, Date.now())
    this.store()
  }

  /** Some da lista (o arquivo foi apagado ou movido). */
  remove(path: string) {
    this.list = this.list.filter((r) => r.path !== path)
    this.store()
  }

  private store() {
    try {
      localStorage.setItem(KEY, JSON.stringify(this.list))
    } catch {
      // sem armazenamento: a lista vale só até fechar
    }
  }
}

export const recents = new Recents()

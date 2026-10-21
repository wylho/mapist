// Atualização do app, como no Ideario: procura uma vez, ao abrir (alguns segundos depois, sem atrasar o mapa), e de
// novo só quando a pessoa pede nas Configurações. O aviso aparece no canto; "Depois" esconde até a próxima abertura.
import { api, hasHost, isPreview, type UpdateInfo } from './api'

class Updates {
  info = $state<UpdateInfo | null>(null)
  phase = $state<'idle' | 'checking' | 'downloading' | 'error'>('idle')
  /** 0–1 enquanto baixa; `null` se o tamanho não é conhecido. */
  progress = $state<number | null>(null)
  /** O aviso do canto foi fechado ("Depois"). As Configurações continuam oferecendo a versão. */
  dismissed = $state(false)

  /** Procura versão nova. Devolve se achou; erro de rede conta como "sem novidade" (tenta na próxima abertura). */
  async check(): Promise<boolean> {
    if (this.phase === 'checking' || this.phase === 'downloading') return !!this.info
    this.phase = 'checking'
    try {
      this.info = await api.checkUpdate()
    } catch {
      this.info = null
    }
    this.phase = 'idle'
    return !!this.info
  }

  /** Baixa, instala e reinicia. Se der errado, o aviso volta com "Tentar de novo". */
  async install() {
    if (!this.info || this.phase === 'downloading') return
    this.phase = 'downloading'
    this.progress = null
    this.dismissed = false
    try {
      await api.installUpdate((done, total) => (this.progress = total ? done / total : null))
    } catch {
      this.phase = 'error'
    }
  }
}

export const updates = new Updates()

if (hasHost || isPreview) setTimeout(() => void updates.check(), 5000)

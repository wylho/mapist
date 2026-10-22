// O card aberto: um tópico ou uma nota solta, um por vez, na janela do card (o mesmo card do Ideario). As notas
// soltas vivem no arquivo (`snapshot.notes`) e ficam no mapa como o card padrão.

/** Um card do mapa: um tópico ou uma nota solta. */
export type NoteRef = { kind: 'node' | 'free'; id: number }

class Notes {
  /** O nó aberto como card, ou null. */
  opened = $state<number | null>(null)
  /** A nota solta aberta como card, ou null (um card aberto por vez). */
  openedFree = $state<number | null>(null)
  /** Abriu para escrever já (tópico novo ou "Adicionar nota"). */
  focus = $state(false)
  /** Modo documento (página inteira), lembrado nesta sessão, como no Ideario. */
  docMode = $state(false)

  isOpen(id: number) {
    return this.opened === id
  }

  /** Abre o nó como card; `edit` já põe o cursor no texto. */
  show(id: number, edit = false) {
    this.openedFree = null
    this.opened = id
    this.focus = edit
  }

  /** Abre a nota solta como card (o card padrão, como um tópico). */
  showFree(id: number, edit = false) {
    this.opened = null
    this.openedFree = id
    this.focus = edit
  }

  closeFree() {
    this.openedFree = null
  }

  toggle(id: number) {
    if (this.isOpen(id)) this.close(id)
    else this.show(id)
  }

  close(id?: number) {
    if (id === undefined || this.opened === id) this.opened = null
  }
}

export const notes = new Notes()

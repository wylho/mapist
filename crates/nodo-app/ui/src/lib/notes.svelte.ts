// O card aberto e as notas soltas. Um nó se abre como o card do Ideario (um por vez, na janela do card); as notas
// soltas vivem no arquivo (`snapshot.notes`), estão sempre à vista e só uma por vez está em edição.

/** Qual card está em edição: a nota solta `id` (o nó aberto é `notes.opened`). */
export type NoteRef = { kind: 'node' | 'free'; id: number }

class Notes {
  /** O nó aberto como card, ou null. */
  opened = $state<number | null>(null)
  /** Abriu para escrever já (tópico novo ou "Adicionar nota"). */
  focus = $state(false)
  /** Modo documento (página inteira), lembrado nesta sessão, como no Ideario. */
  docMode = $state(false)
  editing = $state<NoteRef | null>(null)

  isOpen(id: number) {
    return this.opened === id
  }

  /** Abre o nó como card; `edit` já põe o cursor no texto. */
  show(id: number, edit = false) {
    this.opened = id
    this.focus = edit
  }

  toggle(id: number) {
    if (this.isOpen(id)) this.close(id)
    else this.show(id)
  }

  close(id?: number) {
    if (id === undefined || this.opened === id) this.opened = null
  }

  isEditing(kind: NoteRef['kind'], id: number) {
    return this.editing?.kind === kind && this.editing.id === id
  }
}

export const notes = new Notes()

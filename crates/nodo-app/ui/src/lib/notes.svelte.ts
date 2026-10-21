// Cards de nota no mapa. A nota de um nó abre ao lado dele (clicar na bolinha, ou pelo menu), como um comentário:
// vários podem ficar abertos, cada um lembra o tamanho que a pessoa deu, só nesta sessão. As notas soltas vivem no
// arquivo (`snapshot.notes`) e estão sempre à vista. Só um card por vez está em edição.

export interface OpenNote {
  id: number
  width: number
  height: number
}

/** Qual card está em edição: a nota do nó `id` ou a nota solta `id`. */
export type NoteRef = { kind: 'node' | 'free'; id: number }

class Notes {
  open = $state<OpenNote[]>([])
  editing = $state<NoteRef | null>(null)

  isOpen(id: number) {
    return this.open.some((n) => n.id === id)
  }

  /** Abre a nota do nó; `edit` já entra escrevendo (nota nova). */
  show(id: number, edit = false) {
    if (!this.isOpen(id)) this.open = [...this.open, { id, width: 300, height: 220 }]
    if (edit) this.editing = { kind: 'node', id }
  }

  toggle(id: number) {
    if (this.isOpen(id)) this.close(id)
    else this.show(id)
  }

  close(id: number) {
    this.open = this.open.filter((n) => n.id !== id)
    if (this.isEditing('node', id)) this.editing = null
  }

  isEditing(kind: NoteRef['kind'], id: number) {
    return this.editing?.kind === kind && this.editing.id === id
  }

  resize(id: number, width: number, height: number) {
    const n = this.open.find((x) => x.id === id)
    if (n && (n.width !== width || n.height !== height)) {
      n.width = width
      n.height = height
    }
  }
}

export const notes = new Notes()

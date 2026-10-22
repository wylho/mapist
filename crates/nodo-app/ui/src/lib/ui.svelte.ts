// Estado da interface que não é do documento: folhas abertas, aviso e confirmação.
// Mesmo formato do `app.say` / `app.confirm` do Ideario, para os apps se comportarem igual.

export interface ConfirmRequest {
  title: string
  text: string
  confirm: string
  onconfirm: () => void
  oncancel?: () => void
  /** Terceira saída, a principal ("Salvar"): fechar um mapa com algo por salvar oferece salvar ali mesmo. */
  save?: string
  onsave?: () => void
}

/** O que a pessoa escolheu na caixa de três saídas. */
export type Choice = 'save' | 'discard' | 'cancel'

class Ui {
  settingsOpen = $state(false)
  /** Seção das Configurações para mostrar ao abrir (o botão ? do canto leva aos Atalhos). */
  settingsSection = $state<'atalhos' | null>(null)
  /** Folha Tema do mapa aberta (pela barra ou pelo menu de contexto). */
  themeOpen = $state(false)
  /** Folha Exportar aberta. */
  exportOpen = $state(false)
  /** Busca aberta na barra (no desktop ela é um ícone até ser usada). */
  searchOpen = $state(false)
  /** O Espaço está apertado (Espaço + arrastar move o mapa); `spacePanned` diz se moveu, e aí soltar não edita. */
  spaceHeld = $state(false)
  spacePanned = false
  toast = $state<string | null>(null)
  confirmRequest = $state<ConfirmRequest | null>(null)
  private toastTimer: ReturnType<typeof setTimeout> | undefined

  /** Aviso curto embaixo da tela; some sozinho. */
  say(message: string, ms = 3200) {
    this.toast = message
    clearTimeout(this.toastTimer)
    this.toastTimer = setTimeout(() => (this.toast = null), ms)
  }

  /** Confirmação só para o que não tem volta (o resto usa Desfazer). */
  confirm(req: ConfirmRequest) {
    this.confirmRequest = req
  }

  /** O mesmo, como promessa: `true` se confirmou. */
  ask(req: Omit<ConfirmRequest, 'onconfirm' | 'oncancel'>): Promise<boolean> {
    return new Promise((resolve) =>
      this.confirm({ ...req, onconfirm: () => resolve(true), oncancel: () => resolve(false) }),
    )
  }

  /** Salvar, descartar ou cancelar (fechar um mapa ou o app com algo por salvar). */
  choose(req: { title: string; text: string; save: string; discard: string }): Promise<Choice> {
    return new Promise((resolve) =>
      this.confirm({
        title: req.title,
        text: req.text,
        confirm: req.discard,
        save: req.save,
        onconfirm: () => resolve('discard'),
        onsave: () => resolve('save'),
        oncancel: () => resolve('cancel'),
      }),
    )
  }
}

export const ui = new Ui()

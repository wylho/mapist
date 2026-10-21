// Estado da interface que não é do documento: folhas abertas, aviso e confirmação.
// Mesmo formato do `app.say` / `app.confirm` do Ideario, para os apps se comportarem igual.

export interface ConfirmRequest {
  title: string
  text: string
  confirm: string
  onconfirm: () => void
  oncancel?: () => void
}

class Ui {
  settingsOpen = $state(false)
  /** Folha Tema do mapa aberta (pela barra ou pelo menu de contexto). */
  themeOpen = $state(false)
  /** Busca aberta na barra (no desktop ela é um ícone até ser usada). */
  searchOpen = $state(false)
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
}

export const ui = new Ui()

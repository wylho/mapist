import '@wylho/atrio/atrio.css'
import './map.css'
import { mount } from 'svelte'
import './lib/theme.svelte' // instancia o tema e aplica as variáveis antes da primeira pintura
import App from './App.svelte'

// Um app, não um site: sem o menu do navegador (Voltar, Recarregar, Inspecionar), como no Ideario.
// Campos de texto mantêm o menu nativo (copiar, colar, corretor ortográfico).
document.addEventListener('contextmenu', (e) => {
  if ((e.target as Element).closest?.('input, textarea, [contenteditable="true"]')) return
  e.preventDefault()
})

const target = document.getElementById('app')
if (!target) throw new Error('#app não encontrado no documento')

export default mount(App, { target })

// macOS (como no Ideario): a barra do app é a barra de título. Em tela cheia o sistema esconde os botões da janela e o
// espaço guardado para eles some junto; a largura desse espaço é medida no AppKit, porque muda entre versões.
if (document.documentElement.dataset.titlebar === 'overlay') {
  void import('@tauri-apps/api/window').then(({ getCurrentWindow }) => {
    const w = getCurrentWindow()
    const sync = () =>
      w.isFullscreen().then((f) => document.documentElement.toggleAttribute('data-fullscreen', f)).catch(() => {})
    void sync()
    void w.onResized(sync)
  })
  void import('@tauri-apps/api/core').then(({ invoke }) =>
    invoke<number | null>('mac_lights_width')
      .then((px) => {
        if (px) document.documentElement.style.setProperty('--lights-w', `${Math.round(px)}px`)
      })
      .catch(() => {}),
  )
}

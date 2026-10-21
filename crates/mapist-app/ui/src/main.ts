import './ideario.css'
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

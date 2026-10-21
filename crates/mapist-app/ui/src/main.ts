import { mount } from 'svelte'
import './app.css'
import './lib/theme.svelte' // instancia o tema e aplica as variáveis antes da primeira pintura
import App from './App.svelte'

const target = document.getElementById('app')
if (!target) throw new Error('#app não encontrado no documento')

export default mount(App, { target })

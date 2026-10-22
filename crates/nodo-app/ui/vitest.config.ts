import { defineConfig } from 'vitest/config'
import { svelte } from '@sveltejs/vite-plugin-svelte'

export default defineConfig({
  // O plugin compila os módulos .svelte.ts (runas $state), para testar os stores de verdade.
  plugins: [svelte()],
  resolve: { conditions: ['browser'] },
  test: {
    environment: 'jsdom',
    include: ['tests/**/*.test.ts'],
    // Os testes falam do mapa de exemplo em português: o idioma fica fixo, seja qual for o do sistema.
    setupFiles: ['tests/setup.ts'],
  },
})

// Prints do app para o site (site/img), tirados da prévia no navegador com o mapa de exemplo de src/lib/preview.ts,
// que é inventado (o repositório é público).
//   pnpm build && pnpm site-shots
// Chromium: o do Playwright, ou CHROMIUM=/caminho/do/chrome.
import { spawn, execFileSync } from 'node:child_process'
import { mkdirSync, rmSync } from 'node:fs'
import { fileURLToPath } from 'node:url'
import { chromium } from 'playwright-core'

const PORT = 4189
const OUT = fileURLToPath(new URL('../../../../site/img', import.meta.url))
const server = spawn('pnpm', ['exec', 'vite', 'preview', '--port', String(PORT), '--strictPort'], { stdio: 'ignore' })
const sleep = (ms) => new Promise((r) => setTimeout(r, ms))

async function waitServer() {
  for (let i = 0; i < 100; i++) {
    try {
      if ((await fetch(`http://localhost:${PORT}`)).ok) return
    } catch {}
    await sleep(100)
  }
  throw new Error('prévia não subiu')
}

/** Salva em WebP (bem menor que PNG para o site). */
function webp(png, name) {
  execFileSync('convert', [png, '-quality', '86', '-define', 'webp:method=6', `${OUT}/${name}.webp`])
  rmSync(png)
}

async function shot(browser, name, { width = 1440, height = 900, scale = 2, dark = false, lines = 'curved', query = '' }, act) {
  const ctx = await browser.newContext({
    viewport: { width, height },
    deviceScaleFactor: scale,
    locale: 'pt-BR',
    colorScheme: dark ? 'dark' : 'light',
  })
  await ctx.addInitScript((l) => localStorage.setItem('nodo.ui.linkStyle', JSON.stringify(l)), lines)
  const page = await ctx.newPage()
  await page.goto(`http://localhost:${PORT}/?preview${query}`)
  await page.locator('canvas').waitFor()
  await sleep(600)
  await act?.(page)
  await sleep(500) // animações
  // A etiqueta "Prévia" é para quem desenvolve, não para o site.
  await page.addStyleTag({ content: '.preview-tag{display:none!important}' })
  const png = `${OUT}/${name}.png`
  await page.screenshot({ path: png })
  webp(png, name)
  await ctx.close()
  console.log('  ✓', name)
}

try {
  await waitServer()
  mkdirSync(OUT, { recursive: true })
  const browser = await chromium.launch(process.env.CHROMIUM ? { executablePath: process.env.CHROMIUM } : {})
  await shot(browser, 'mapa', { query: '&select=Servidor%20MCP' })
  await shot(browser, 'mapa-escuro', { dark: true, lines: 'orthogonal', query: '&select=Servidor%20MCP' })
  await shot(browser, 'temas', {}, async (page) => {
    await page.getByRole('button', { name: 'Tema do mapa' }).click()
    await page.getByRole('radio', { name: /Sublinhado/ }).click()
    await sleep(400)
  })
  await shot(browser, 'busca', {}, async (page) => {
    await page.getByRole('button', { name: 'Buscar' }).click()
    await page.locator('#busca').fill('a')
    await page.locator('#busca').fill('Design')
  })
  await shot(browser, 'claude', {}, async (page) => {
    await page.getByRole('button', { name: 'Configurações' }).click()
    await page.locator('.sheet').waitFor()
    await page.locator('.sheet-scroll').evaluate((el) => (el.scrollTop = el.scrollHeight))
  })
  await shot(browser, 'janela-estreita', { width: 390, height: 844, scale: 3, dark: true })
  await browser.close()
} finally {
  server.kill()
}

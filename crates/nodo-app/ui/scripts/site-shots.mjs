// Prints do app para o site, tirados da prévia no navegador com o mapa de exemplo de src/lib/preview.ts, que é
// inventado (o repositório é público). Em inglês em site/img (a página principal) e em português em site/img/pt.
// Regra do usuário: a tela cheia de cards, nunca um app vazio; no recorte de celular, zoom nos cards.
//   pnpm build && pnpm site-shots
// Chromium: o do Playwright, ou CHROMIUM=/caminho/do/chrome.
import { spawn, execFileSync } from 'node:child_process'
import { mkdirSync, rmSync } from 'node:fs'
import { fileURLToPath } from 'node:url'
import { chromium } from 'playwright-core'

const PORT = 4189
const IMG = fileURLToPath(new URL('../../../../site/img', import.meta.url))
let OUT = IMG
let LOCALE = 'en'
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
    locale: LOCALE,
    colorScheme: dark ? 'dark' : 'light',
  })
  await ctx.addInitScript(([l, loc]) => {
    localStorage.setItem('nodo.ui.linkStyle', JSON.stringify(l))
    localStorage.setItem('atrio.locale', loc)
  }, [lines, LOCALE])
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
  const browser = await chromium.launch(process.env.CHROMIUM ? { executablePath: process.env.CHROMIUM } : {})
  for (const [locale, dir] of [['en', IMG], ['pt-BR', `${IMG}/pt`]]) {
    LOCALE = locale
    OUT = dir
    mkdirSync(OUT, { recursive: true })
    const pt = locale === 'pt-BR'
    const L = (p, e) => (pt ? p : e)
    const mcp = encodeURIComponent(L('Servidor MCP', 'MCP server'))
    await shot(browser, 'mapa', { query: `&select=${mcp}` })
    await shot(browser, 'mapa-escuro', { dark: true, lines: 'orthogonal', query: `&select=${mcp}` })
    await shot(browser, 'temas', {}, async (page) => {
      await page.getByRole('button', { name: L('Estrutura e tema do mapa', 'Map structure and theme') }).click()
      await page.getByRole('radio', { name: new RegExp(L('Sublinhado', 'Underline')) }).click()
      await sleep(400)
    })
    await shot(browser, 'busca', {}, async (page) => {
      await page.getByRole('button', { name: L('Buscar', 'Search'), exact: true }).click()
      await page.locator('#busca').fill('a')
      await page.locator('#busca').fill('Design')
    })
    await shot(browser, 'claude', {}, async (page) => {
      await page.getByRole('button', { name: L('Configurações', 'Settings') }).click()
      await page.locator('.sheet').waitFor()
      // A seção do Claude no alto da folha (a lista de atalhos vem depois dela).
      await page.locator('.sheet h3', { hasText: /^Claude$/ }).evaluate((el) => el.scrollIntoView({ block: 'start' }))
    })
    // Celular: zoom no card da Construção, com os vizinhos em volta (nada de mapa miúdo).
    const build = encodeURIComponent(L('Construção', 'Build'))
    await shot(browser, 'janela-estreita', { width: 390, height: 844, scale: 3, dark: true, query: `&select=${build}` }, async (page) => {
      await page.keyboard.press('Control+Digit0')
      await sleep(400)
      // Um pouco menos que 100%, ancorado à esquerda do card: ele e os subtópicos dele cabem inteiros.
      await page.mouse.move(60, 470)
      for (let i = 0; i < 2; i++) {
        await page.mouse.wheel(0, 100)
        await sleep(150)
      }
    })
  }
  await browser.close()
} finally {
  server.kill()
}

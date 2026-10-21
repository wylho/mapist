// Markdown das notas em HTML seguro, para ler a nota formatada no balão. O HTML passa pelo DOMPurify: uma nota pode
// ter vindo de um arquivo de outra pessoa, e nada nela pode rodar script dentro do app.
import { marked } from 'marked'
import DOMPurify from 'dompurify'

export function renderMarkdown(text: string): string {
  const html = marked.parse(text, { async: false, gfm: true, breaks: true }) as string
  return DOMPurify.sanitize(html, { USE_PROFILES: { html: true } })
}

/** Abre um link da nota no navegador do sistema (no app) ou numa aba nova (na prévia), nunca dentro da janela. */
export async function openLink(href: string) {
  if (!/^(https?:|mailto:)/i.test(href)) return
  if ('__TAURI_INTERNALS__' in window) {
    const { openUrl } = await import('@tauri-apps/plugin-opener')
    await openUrl(href)
  } else {
    window.open(href, '_blank', 'noopener')
  }
}

// Fonte para o canvas 2D.
//
// Regra que evita o bug mais fácil de cometer aqui: a fonte é sempre expressa
// em unidades de MUNDO, nunca em pixels de tela. O zoom entra por
// `ctx.scale()`, que escala tudo junto — geometria, traço e texto.
//
// Medir em tamanho cheio e desenhar em tamanho reduzido (ou o contrário) faz o
// texto não caber na própria caixa e ser truncado, que foi exatamente o defeito
// observado: nós medidos a 13.5px e desenhados a 51%, todos cortados.

let cached: string | null = null

/**
 * A pilha de fontes concreta, lida de um elemento real para que o navegador a
 * resolva: `ctx.font` não entende `var()` e cairia na fonte padrão.
 */
export function fontStack(): string {
  if (cached) return cached
  const probe = document.createElement('span')
  probe.style.fontFamily = 'var(--font-body)'
  probe.style.position = 'absolute'
  probe.style.visibility = 'hidden'
  probe.style.pointerEvents = 'none'
  document.body.appendChild(probe)
  const resolved = getComputedStyle(probe).fontFamily.trim()
  probe.remove()
  cached = resolved || 'system-ui, -apple-system, sans-serif'
  return cached
}

/** Esquece a pilha memorizada. Chamar quando o tema trocar. */
export function resetFontStack() {
  cached = null
}

/** Tamanho do texto de um nó, em unidades de mundo. */
export const NODE_FONT_SIZE = 13.5
/** Tamanho do texto da raiz, em unidades de mundo. */
export const ROOT_FONT_SIZE = 14

/** `ctx.font` para um nó, em unidades de mundo. */
export function nodeFont(isRoot: boolean): string {
  return `${isRoot ? 700 : 600} ${isRoot ? ROOT_FONT_SIZE : NODE_FONT_SIZE}px ${fontStack()}`
}

/** `ctx.font` para um nível do tema do mapa, em unidades de mundo. */
export function levelFont(style: { size: number; weight: number }): string {
  return `${style.weight} ${style.size}px ${fontStack()}`
}

/** `ctx.font` para textos de interface no canvas, em unidades de mundo. */
export function uiFont(size: number): string {
  return `${size}px ${fontStack()}`
}

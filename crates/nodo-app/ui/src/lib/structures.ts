// Estruturas de mapa e espaçamentos (os mesmos nomes de nodo_core::STRUCTURES e SPACINGS). O desenho de cada
// estrutura é um esquema pequeno em SVG, para a escolha ser visual como no XMind.

export const STRUCTURES = [
  { id: 'mapa', label: 'Mapa', hint: 'Ramos dos dois lados' },
  { id: 'logico', label: 'Lógico', hint: 'Tudo para a direita' },
  { id: 'organograma', label: 'Organograma', hint: 'De cima para baixo' },
  { id: 'arvore', label: 'Árvore', hint: 'Lista recuada' },
] as const

export type StructureId = (typeof STRUCTURES)[number]['id']

export const SPACINGS = [
  { id: 'compacto', label: 'Compacto' },
  { id: 'normal', label: 'Normal' },
  { id: 'largo', label: 'Largo' },
] as const

/** A estrutura atual de um snapshot (null é o mapa). */
export const structureOf = (s: string | null): StructureId =>
  (STRUCTURES.find((x) => x.id === s)?.id ?? 'mapa') as StructureId

export const spacingOf = (s: string | null) => SPACINGS.find((x) => x.id === s)?.id ?? 'normal'

/** Esquema da estrutura: caixas (x, y, largura, altura) e linhas (pontos), num quadro de 96 × 56. */
export function sketch(id: StructureId): { boxes: [number, number, number, number][]; lines: string[] } {
  switch (id) {
    case 'logico':
      return {
        boxes: [[6, 23, 22, 10], [44, 6, 20, 8], [44, 24, 20, 8], [44, 42, 20, 8], [72, 2, 18, 7], [72, 13, 18, 7]],
        lines: ['M28 28 C36 28 36 10 44 10', 'M28 28 L44 28', 'M28 28 C36 28 36 46 44 46', 'M64 10 C68 10 68 5 72 5', 'M64 10 C68 10 68 16 72 16'],
      }
    case 'organograma':
      return {
        boxes: [[37, 4, 22, 10], [8, 26, 20, 8], [38, 26, 20, 8], [68, 26, 20, 8], [4, 44, 14, 7], [20, 44, 14, 7]],
        lines: ['M48 14 C48 20 18 20 18 26', 'M48 14 L48 26', 'M48 14 C48 20 78 20 78 26', 'M18 34 C18 39 11 39 11 44', 'M18 34 C18 39 27 39 27 44'],
      }
    case 'arvore':
      return {
        boxes: [[8, 3, 30, 8], [22, 15, 28, 7], [36, 25, 26, 7], [36, 35, 26, 7], [22, 45, 28, 7]],
        lines: ['M14 11 L14 18.5 L22 18.5', 'M28 22 L28 28.5 L36 28.5', 'M28 22 L28 38.5 L36 38.5', 'M14 11 L14 48.5 L22 48.5'],
      }
    default:
      return {
        boxes: [[36, 23, 24, 10], [6, 8, 20, 8], [6, 40, 20, 8], [70, 6, 20, 8], [70, 24, 20, 8], [70, 42, 20, 8]],
        lines: ['M36 28 C30 28 30 12 26 12', 'M36 28 C30 28 30 44 26 44', 'M60 28 C66 28 64 10 70 10', 'M60 28 L70 28', 'M60 28 C66 28 64 46 70 46'],
      }
  }
}

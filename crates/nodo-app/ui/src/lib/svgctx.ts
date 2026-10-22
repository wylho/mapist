// Um "canvas" que escreve SVG. O desenho do mapa (render.ts) usa um pedaço pequeno da API do canvas; este objeto
// implementa esse pedaço e grava elementos SVG em vez de pixels. Assim o SVG exportado sai do mesmo código que desenha
// a tela, e as duas coisas nunca divergem.

interface State {
  fillStyle: string
  strokeStyle: string
  lineWidth: number
  lineCap: string
  globalAlpha: number
  font: string
  textAlign: CanvasTextAlign
  textBaseline: CanvasTextBaseline
  transform: [number, number, number, number, number, number]
  /** Recorte em vigor (id de um `<clipPath>`), até o `restore`. */
  clip: string | null
}

const fmt = (n: number) => (Math.round(n * 100) / 100).toString()

const esc = (s: string) => s.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;').replace(/"/g, '&quot;')

export class SvgContext {
  private parts: string[] = []
  private d = ''
  private cur: [number, number] | null = null
  private start: [number, number] | null = null
  private stack: State[] = []
  private s: State = {
    fillStyle: '#000',
    strokeStyle: '#000',
    lineWidth: 1,
    lineCap: 'butt',
    globalAlpha: 1,
    font: '10px sans-serif',
    textAlign: 'start',
    textBaseline: 'alphabetic',
    transform: [1, 0, 0, 1, 0, 0],
    clip: null,
  }
  private clips = 0
  // Sombras não vão para o SVG (o arquivo fica limpo e leve); as propriedades existem para o desenho não quebrar.
  shadowColor = 'transparent'
  shadowBlur = 0
  shadowOffsetX = 0
  shadowOffsetY = 0

  get fillStyle() { return this.s.fillStyle }
  set fillStyle(v: string) { this.s.fillStyle = v }
  get strokeStyle() { return this.s.strokeStyle }
  set strokeStyle(v: string) { this.s.strokeStyle = v }
  get lineWidth() { return this.s.lineWidth }
  set lineWidth(v: number) { this.s.lineWidth = v }
  get lineCap() { return this.s.lineCap }
  set lineCap(v: string) { this.s.lineCap = v }
  get globalAlpha() { return this.s.globalAlpha }
  set globalAlpha(v: number) { this.s.globalAlpha = v }
  get font() { return this.s.font }
  set font(v: string) { this.s.font = v }
  get textAlign() { return this.s.textAlign }
  set textAlign(v: CanvasTextAlign) { this.s.textAlign = v }
  get textBaseline() { return this.s.textBaseline }
  set textBaseline(v: CanvasTextBaseline) { this.s.textBaseline = v }

  save() { this.stack.push({ ...this.s, transform: [...this.s.transform] as State['transform'] }) }
  restore() { const s = this.stack.pop(); if (s) this.s = s }
  setTransform(a: number, b: number, c: number, d: number, e: number, f: number) { this.s.transform = [a, b, c, d, e, f] }
  clearRect() {}

  beginPath() { this.d = ''; this.cur = null; this.start = null }
  moveTo(x: number, y: number) { this.d += `M${fmt(x)} ${fmt(y)}`; this.cur = [x, y]; this.start = [x, y] }
  lineTo(x: number, y: number) {
    if (!this.cur) return this.moveTo(x, y)
    this.d += `L${fmt(x)} ${fmt(y)}`
    this.cur = [x, y]
  }
  bezierCurveTo(c1x: number, c1y: number, c2x: number, c2y: number, x: number, y: number) {
    if (!this.cur) this.moveTo(c1x, c1y)
    this.d += `C${fmt(c1x)} ${fmt(c1y)} ${fmt(c2x)} ${fmt(c2y)} ${fmt(x)} ${fmt(y)}`
    this.cur = [x, y]
  }
  closePath() { if (this.start) { this.d += 'Z'; this.cur = this.start } }

  /** Como o `arcTo` do canvas: reta até o ponto de tangência e arco de raio `r` virando para (x2, y2). */
  arcTo(x1: number, y1: number, x2: number, y2: number, r: number) {
    if (!this.cur) return this.moveTo(x1, y1)
    const [x0, y0] = this.cur
    const u1 = [x0 - x1, y0 - y1]
    const u2 = [x2 - x1, y2 - y1]
    const l1 = Math.hypot(u1[0], u1[1])
    const l2 = Math.hypot(u2[0], u2[1])
    if (r <= 0 || l1 === 0 || l2 === 0) return this.lineTo(x1, y1)
    u1[0] /= l1; u1[1] /= l1; u2[0] /= l2; u2[1] /= l2
    const cos = Math.max(-1, Math.min(1, u1[0] * u2[0] + u1[1] * u2[1]))
    const theta = Math.acos(cos)
    if (theta < 1e-6 || Math.PI - theta < 1e-6) return this.lineTo(x1, y1)
    const dist = r / Math.tan(theta / 2)
    const t1 = [x1 + u1[0] * dist, y1 + u1[1] * dist]
    const t2 = [x1 + u2[0] * dist, y1 + u2[1] * dist]
    // Sentido da curva: de onde vem (−u1) para onde vai (u2); positivo vira no sentido horário (y para baixo).
    const turn = -u1[0] * u2[1] + u1[1] * u2[0]
    this.lineTo(t1[0], t1[1])
    this.d += `A${fmt(r)} ${fmt(r)} 0 0 ${turn > 0 ? 1 : 0} ${fmt(t2[0])} ${fmt(t2[1])}`
    this.cur = [t2[0], t2[1]]
  }

  arc(x: number, y: number, r: number, a0: number, a1: number, ccw = false) {
    const p = (a: number): [number, number] => [x + r * Math.cos(a), y + r * Math.sin(a)]
    const [sx, sy] = p(a0)
    if (this.cur) this.lineTo(sx, sy)
    else this.moveTo(sx, sy)
    let delta = ccw ? a0 - a1 : a1 - a0
    if (delta >= Math.PI * 2 - 1e-9) {
      const [mx, my] = p(a0 + (ccw ? -Math.PI : Math.PI))
      const sweep = ccw ? 0 : 1
      this.d += `A${fmt(r)} ${fmt(r)} 0 1 ${sweep} ${fmt(mx)} ${fmt(my)}A${fmt(r)} ${fmt(r)} 0 1 ${sweep} ${fmt(sx)} ${fmt(sy)}`
      this.cur = [sx, sy]
      return
    }
    delta = ((delta % (Math.PI * 2)) + Math.PI * 2) % (Math.PI * 2)
    const [ex, ey] = p(a1)
    this.d += `A${fmt(r)} ${fmt(r)} 0 ${delta > Math.PI ? 1 : 0} ${ccw ? 0 : 1} ${fmt(ex)} ${fmt(ey)}`
    this.cur = [ex, ey]
  }

  private attrs(): string {
    const [a, b, c, d, e, f] = this.s.transform
    const t = a === 1 && b === 0 && c === 0 && d === 1 && e === 0 && f === 0 ? '' : ` transform="matrix(${[a, b, c, d, e, f].map(fmt).join(' ')})"`
    const o = this.s.globalAlpha < 1 ? ` opacity="${fmt(this.s.globalAlpha)}"` : ''
    const cl = this.s.clip ? ` clip-path="url(#${this.s.clip})"` : ''
    return t + o + cl
  }

  fill() {
    if (this.d) this.parts.push(`<path d="${this.d}" fill="${esc(this.s.fillStyle)}"${this.attrs()}/>`)
  }

  stroke() {
    if (!this.d) return
    this.parts.push(
      `<path d="${this.d}" fill="none" stroke="${esc(this.s.strokeStyle)}" stroke-width="${fmt(this.s.lineWidth)}" stroke-linecap="${this.s.lineCap}" stroke-linejoin="round"${this.attrs()}/>`,
    )
  }

  fillText(text: string, x: number, y: number) {
    // "600 13px Inter, system-ui" → peso, tamanho e família.
    const m = /^(?:(italic)\s+)?(?:(\d{3}|bold|normal)\s+)?([\d.]+)px\s+(.+)$/.exec(this.s.font.trim())
    const weight = m?.[2] ?? '400'
    const size = m?.[3] ?? '10'
    const family = m?.[4] ?? 'sans-serif'
    const anchor = this.s.textAlign === 'center' ? 'middle' : this.s.textAlign === 'right' || this.s.textAlign === 'end' ? 'end' : 'start'
    const base =
      this.s.textBaseline === 'middle' ? 'central'
      : this.s.textBaseline === 'top' || this.s.textBaseline === 'hanging' ? 'hanging'
      : 'auto'
    this.parts.push(
      `<text x="${fmt(x)}" y="${fmt(y)}" font-family="${esc(family)}" font-size="${size}" font-weight="${weight}" fill="${esc(this.s.fillStyle)}" text-anchor="${anchor}" dominant-baseline="${base}"${this.attrs()}>${esc(text)}</text>`,
    )
  }

  /** Recorta o que vier depois pelo caminho atual (o canto arredondado da capa do card). */
  clip() {
    if (!this.d) return
    const id = `c${++this.clips}`
    // Em `userSpaceOnUse` o recorte usa o sistema de quem o recebe, já com a mesma transformação.
    this.parts.push(`<clipPath id="${id}" clipPathUnits="userSpaceOnUse"><path d="${this.d}"/></clipPath>`)
    this.s.clip = id
  }

  /** Uma foto: vai dentro do SVG como `data:` (o `src` da imagem já é um, feito em `domcard.ts`). */
  drawImage(img: HTMLImageElement, x: number, y: number, w: number, h: number) {
    this.parts.push(
      `<image href="${esc(img.src)}" x="${fmt(x)}" y="${fmt(y)}" width="${fmt(w)}" height="${fmt(h)}" preserveAspectRatio="xMidYMid slice"${this.attrs()}/>`,
    )
  }

  /** Um ícone em SVG, como vetor (não como imagem). */
  drawSvgMarkup(markup: string, x: number, y: number, w: number, h: number) {
    // O ícone já vem com a largura e a altura do lugar (`domcard.ts`); aqui só vai para a posição.
    void w
    void h
    this.parts.push(`<g${this.attrs()}><g transform="translate(${fmt(x)} ${fmt(y)})">${markup}</g></g>`)
  }

  /** O documento SVG, com fundo opcional. */
  toSvg(width: number, height: number, background: string | null): string {
    const bg = background ? `<rect width="100%" height="100%" fill="${esc(background)}"/>` : ''
    return `<svg xmlns="http://www.w3.org/2000/svg" width="${fmt(width)}" height="${fmt(height)}" viewBox="0 0 ${fmt(width)} ${fmt(height)}">${bg}${this.parts.join('')}</svg>`
  }
}

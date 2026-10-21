<script lang="ts">
  // Configurações, com a mesma estrutura e os mesmos componentes da SettingsSheet do Ideario:
  // Aparência (tema e claro/escuro), Mapa (traçado das linhas) e Claude (MCP). Tudo grava na hora.
  import { Dialog, ToggleGroup } from 'bits-ui'
  import { Copy, Monitor, Moon, Sun, X } from '@lucide/svelte'
  import { untrack } from 'svelte'
  import { api, hasHost, isPreview, type McpInfo } from '../lib/api'
  import { ui } from '../lib/ui.svelte'
  import { prefs } from '../lib/prefs.svelte'
  import { theme, type ThemePref } from '../lib/theme.svelte'
  import { DESKTOPS, PALETTES, paletteColors, type DesktopId, type PaletteId } from '../lib/palettes'
  import type { LinkStyle } from '../lib/render'
  import Picker from './Picker.svelte'

  // Sistema primeiro: é o padrão e faz o app parecer nativo.
  const THEMES: { id: PaletteId; label: string; hint: string }[] = [
    { id: 'system', label: 'Sistema', hint: '' },
    ...Object.entries(PALETTES).map(([id, p]) => ({ id: id as PaletteId, label: p.label, hint: p.hint })),
  ]
  const systemName = $derived(`Cores do ${DESKTOPS[theme.simDesktop].label}`)

  let version = $state<string | null>(null)
  if (hasHost) {
    void import('@tauri-apps/api/app').then(({ getVersion }) => getVersion().then((v) => (version = v)))
  }

  // ---------- Claude (MCP) ----------
  let mcp = $state.raw<McpInfo | null>(null)
  $effect(() => {
    if (ui.settingsOpen) untrack(() => api.mcpInfo().then((m) => (mcp = m)).catch(() => (mcp = null)))
  })
  async function mcpToggle() {
    if (!mcp) return
    const on = !mcp.desktopInstalled
    try {
      mcp = await (on ? api.mcpInstall() : api.mcpUninstall())
      ui.say(on ? 'Ligado no Claude Desktop. Se ele estiver aberto, feche e abra de novo.' : 'Desligado do Claude Desktop.')
    } catch (e) {
      ui.say(`Não foi possível: ${e instanceof Error ? e.message : e}`)
    }
  }
  async function copyCommand() {
    if (!mcp) return
    try {
      await navigator.clipboard.writeText(mcp.claudeCode)
      ui.say('Comando copiado. Cole no terminal.')
    } catch {
      ui.say(mcp.claudeCode)
    }
  }
</script>

<Dialog.Root bind:open={ui.settingsOpen}>
  <Dialog.Portal>
    <Dialog.Overlay class="overlay" />
    <Dialog.Content class="sheet" aria-describedby={undefined}>
      <div class="sheet-head">
        <Dialog.Title class="sheet-title">Configurações</Dialog.Title>
        <Dialog.Close class="icon-btn" aria-label="Fechar"><X size={20} /></Dialog.Close>
      </div>
      <div class="sheet-scroll">
        <section class="set-group">
          <h3>Aparência</h3>
          <div class="theme-grid" role="radiogroup" aria-label="Tema">
            {#each THEMES as t (t.id)}
              {@const c = paletteColors(t.id, theme.mode, theme.simDesktop, theme.systemAccent)}
              <button class="theme-tile" role="radio" aria-checked={theme.palette === t.id} onclick={() => theme.setPalette(t.id)}>
                <span class="theme-swatch" style:background={c.bg} aria-hidden="true">
                  <span class="theme-card" style:background={c.surface} style:border-color={c.line}>
                    <i style:background={c.fg}></i><i style:background={c.muted}></i>
                  </span>
                  <span class="theme-dots">
                    <i style:background={c.accent}></i>{#each c.notes.slice(0, 3) as n (n)}<i style:background={n}></i>{/each}
                  </span>
                </span>
                <span class="theme-name">{t.label}</span>
                <small>{t.id === 'system' ? systemName : t.hint}</small>
              </button>
            {/each}
          </div>
          {#if theme.palette === 'system' && !hasHost}
            <div class="set-row">
              <span><b>Prévia do sistema</b><small>Só no navegador. No app, as cores vêm da área de trabalho.</small></span>
              <Picker
                id="sistema"
                bind:value={() => theme.simDesktop, (v) => theme.setSimDesktop(v)}
                options={Object.entries(DESKTOPS).map(([id, d]) => [id, d.label]) as [DesktopId, string][]}
              />
            </div>
          {/if}
          <div class="set-row">
            <span><b>Claro ou escuro</b></span>
            <ToggleGroup.Root
              type="single"
              value={theme.pref}
              onValueChange={(v) => v && theme.set(v as ThemePref)}
              class="seg tight"
              aria-label="Claro ou escuro"
            >
              <ToggleGroup.Item value="light" class="seg-item"><Sun size={15} />Claro</ToggleGroup.Item>
              <ToggleGroup.Item value="dark" class="seg-item"><Moon size={15} />Escuro</ToggleGroup.Item>
              <ToggleGroup.Item value="system" class="seg-item"><Monitor size={15} />Automático</ToggleGroup.Item>
            </ToggleGroup.Root>
          </div>
        </section>

        <section class="set-group">
          <h3>Mapa</h3>
          <div class="set-row">
            <span><b>Linhas</b><small>Como os ramos ligam um nó ao outro.</small></span>
            <ToggleGroup.Root
              type="single"
              value={prefs.linkStyle}
              onValueChange={(v) => v && prefs.setLinkStyle(v as LinkStyle)}
              class="seg tight"
              aria-label="Traçado das linhas"
            >
              <ToggleGroup.Item value="curved" class="seg-item">Curvas</ToggleGroup.Item>
              <ToggleGroup.Item value="orthogonal" class="seg-item">Em ângulo</ToggleGroup.Item>
            </ToggleGroup.Root>
          </div>
        </section>

        {#if mcp}
          <section class="set-group">
            <h3>Claude</h3>
            <div class="set-row">
              <span>
                <b>Claude Desktop</b>
                <small>{mcp.desktopInstalled
                  ? 'Ligado: no Claude, peça para ler, criar e reorganizar seus mapas. Tudo continua no seu computador.'
                  : mcp.desktopFound
                    ? 'Deixa o Claude ler, criar e reorganizar seus mapas, pelo mesmo núcleo do app e com o mesmo desfazer. Tudo continua no seu computador.'
                    : 'O Claude Desktop não foi encontrado neste computador. Dá para ligar mesmo assim (vale quando ele for instalado).'}</small>
              </span>
              <button class="btn {mcp.desktopInstalled ? 'ghost' : 'primary'}" onclick={mcpToggle}>{mcp.desktopInstalled ? 'Desligar' : 'Ligar'}</button>
            </div>
            <div class="set-row mcp-code">
              <span>
                <b>Claude Code</b>
                <small>Rode uma vez no terminal:</small>
                <code>{mcp.claudeCode}</code>
              </span>
              <button class="icon-btn" aria-label="Copiar comando" title="Copiar" onclick={copyCommand}><Copy size={18} /></button>
            </div>
          </section>
        {/if}

        <p class="footnote">
          {isPreview ? 'Prévia no navegador · dados de exemplo' : `Mapist${version ? ` v${version}` : ''}`}
        </p>
      </div>
    </Dialog.Content>
  </Dialog.Portal>
</Dialog.Root>

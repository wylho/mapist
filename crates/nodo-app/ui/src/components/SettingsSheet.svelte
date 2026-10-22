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
  import type { DragBackground } from '../lib/prefs.svelte'
  import { i18n, LOCALES, Picker, setLocale, t, type LocalePref } from '@wylho/atrio'
  import { updates } from '../lib/updates.svelte'

  // Sistema primeiro: é o padrão e faz o app parecer nativo.
  const THEMES: { id: PaletteId; label: string; hint: string }[] = [
    { id: 'system', label: 'Sistema', hint: '' },
    ...Object.entries(PALETTES).map(([id, p]) => ({ id: id as PaletteId, label: p.label, hint: p.hint })),
  ]
  const systemName = $derived(t('Cores do {sistema}', { sistema: DESKTOPS[theme.simDesktop].label }))

  const mod = /Mac/.test(navigator.platform) ? '⌘' : 'Ctrl+'
  const alt = /Mac/.test(navigator.platform) ? '⌥' : 'Alt+'

  let version = $state<string | null>(null)
  if (hasHost) {
    void import('@tauri-apps/api/app').then(({ getVersion }) => getVersion().then((v) => (version = v)))
  }

  async function checkNow() {
    if (!(await updates.check())) ui.say(t('Você já tem a versão mais recente.'))
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
      ui.say(on ? t('Ligado no Claude Desktop. Se ele estiver aberto, feche e abra de novo.') : t('Desligado do Claude Desktop.'))
    } catch (e) {
      ui.say(t('Não foi possível: {erro}', { erro: String(e instanceof Error ? e.message : e) }))
    }
  }
  async function copyCommand() {
    if (!mcp) return
    try {
      await navigator.clipboard.writeText(mcp.claudeCode)
      ui.say(t('Comando copiado. Cole no terminal.'))
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
        <Dialog.Title class="sheet-title">{t('Configurações')}</Dialog.Title>
        <Dialog.Close class="icon-btn" aria-label={t('Fechar')}><X size={20} /></Dialog.Close>
      </div>
      <div class="sheet-scroll">
        <section class="set-group">
          <h3>{t('Aparência')}</h3>
          <div class="theme-grid" role="radiogroup" aria-label={t('Tema')}>
            {#each THEMES as th (th.id)}
              {@const c = paletteColors(th.id, theme.mode, theme.simDesktop, theme.systemAccent)}
              <button class="theme-tile" role="radio" aria-checked={theme.palette === th.id} onclick={() => theme.setPalette(th.id)}>
                <span class="theme-swatch" style:background={c.bg} aria-hidden="true">
                  <span class="theme-card" style:background={c.surface} style:border-color={c.line}>
                    <i style:background={c.fg}></i><i style:background={c.muted}></i>
                  </span>
                  <span class="theme-dots">
                    <i style:background={c.accent}></i>{#each c.notes.slice(0, 3) as n (n)}<i style:background={n}></i>{/each}
                  </span>
                </span>
                <span class="theme-name">{t(th.label)}</span>
                <small>{th.id === 'system' ? systemName : t(th.hint)}</small>
              </button>
            {/each}
          </div>
          {#if theme.palette === 'system' && !hasHost}
            <div class="set-row">
              <span><b>{t('Prévia do sistema')}</b><small>{t('Só no navegador. No app, as cores vêm da área de trabalho.')}</small></span>
              <Picker
                id="sistema"
                bind:value={() => theme.simDesktop, (v) => theme.setSimDesktop(v)}
                options={Object.entries(DESKTOPS).map(([id, d]) => [id, d.label]) as [DesktopId, string][]}
              />
            </div>
          {/if}
          <div class="set-row">
            <span><b>{t('Claro ou escuro')}</b></span>
            <ToggleGroup.Root
              type="single"
              value={theme.pref}
              onValueChange={(v) => v && theme.set(v as ThemePref)}
              class="seg tight"
              aria-label={t('Claro ou escuro')}
            >
              <ToggleGroup.Item value="light" class="seg-item"><Sun size={15} />{t('Claro')}</ToggleGroup.Item>
              <ToggleGroup.Item value="dark" class="seg-item"><Moon size={15} />{t('Escuro')}</ToggleGroup.Item>
              <ToggleGroup.Item value="system" class="seg-item"><Monitor size={15} />{t('Automático')}</ToggleGroup.Item>
            </ToggleGroup.Root>
          </div>
          <div class="set-row">
            <span><b>{t('Idioma')}</b><small>{t('Automático segue o idioma do sistema.')}</small></span>
            <ToggleGroup.Root
              type="single"
              value={i18n.pref}
              onValueChange={(v) => v && setLocale(v as LocalePref)}
              class="seg tight"
              aria-label={t('Idioma')}
            >
              {#each LOCALES as l (l.id)}<ToggleGroup.Item value={l.id} class="seg-item">{t(l.label)}</ToggleGroup.Item>{/each}
            </ToggleGroup.Root>
          </div>
        </section>

        <section class="set-group">
          <h3>{t('Mapa')}</h3>
          <div class="set-row">
            <span><b>{t('Linhas')}</b><small>{t('Como os ramos ligam um nó ao outro.')}</small></span>
            <ToggleGroup.Root
              type="single"
              value={prefs.linkStyle}
              onValueChange={(v) => v && prefs.setLinkStyle(v as LinkStyle)}
              class="seg tight"
              aria-label={t('Traçado das linhas')}
            >
              <ToggleGroup.Item value="curved" class="seg-item">{t('Curvas')}</ToggleGroup.Item>
              <ToggleGroup.Item value="orthogonal" class="seg-item">{t('Em ângulo')}</ToggleGroup.Item>
            </ToggleGroup.Root>
          </div>
          <div class="set-row">
            <span
              ><b>{t('Arrastar o fundo')}</b><small
                >{prefs.dragBackground === 'select'
                  ? t('Escolhe os tópicos num retângulo. Para mover o mapa: Espaço + arrastar ou o botão do meio.')
                  : t('Move o mapa. Para escolher tópicos num retângulo: ⇧ + arrastar.')}</small
              ></span
            >
            <ToggleGroup.Root
              type="single"
              value={prefs.dragBackground}
              onValueChange={(v) => v && prefs.setDragBackground(v as DragBackground)}
              class="seg tight"
              aria-label={t('Arrastar o fundo')}
            >
              <ToggleGroup.Item value="select" class="seg-item">{t('Escolher')}</ToggleGroup.Item>
              <ToggleGroup.Item value="pan" class="seg-item">{t('Mover o mapa')}</ToggleGroup.Item>
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
                  ? t('Ligado: no Claude, peça para ler, criar e reorganizar seus mapas. Tudo continua no seu computador.')
                  : mcp.desktopFound
                    ? t('Deixa o Claude ler, criar e reorganizar seus mapas, pelo mesmo núcleo do app e com o mesmo desfazer. Tudo continua no seu computador.')
                    : t('O Claude Desktop não foi encontrado neste computador. Dá para ligar mesmo assim (vale quando ele for instalado).')}</small>
              </span>
              <button class="btn {mcp.desktopInstalled ? 'ghost' : 'primary'}" onclick={mcpToggle}>{mcp.desktopInstalled ? t('Desligar') : t('Ligar')}</button>
            </div>
            <div class="set-row mcp-code">
              <span>
                <b>Claude Code</b>
                <small>{t('Rode uma vez no terminal:')}</small>
                <code>{mcp.claudeCode}</code>
              </span>
              <button class="icon-btn" aria-label={t('Copiar comando')} title={t('Copiar')} onclick={copyCommand}><Copy size={18} /></button>
            </div>
          </section>
        {/if}

        <section class="set-group">
          <h3>{t('Atalhos')}</h3>
            <span class="keys-head">{t('Escrever')}</span>
            <div class="keys">
              <div class="keys-row"><span>{t('Escrever no nó')}</span><kbd>{t('digite')}</kbd></div>
              <div class="keys-row"><span>{t('Editar o texto')}</span><kbd>{t('Espaço ou F2')}</kbd></div>
              <div class="keys-row"><span>{t('Adicionar filho')}</span><kbd>Tab</kbd></div>
              <div class="keys-row"><span>{t('Adicionar irmão')}</span><kbd>Enter</kbd></div>
              <div class="keys-row"><span>{t('Confirmar e criar o próximo')}</span><kbd>{t('Tab (editando)')}</kbd></div>
              <div class="keys-row"><span>{t('Cancelar a edição')}</span><kbd>Esc</kbd></div>
              <div class="keys-row"><span>{t('Abrir o card do tópico')}</span><kbd>{mod}Enter</kbd></div>
            </div>
            <span class="keys-head">{t('Organizar')}</span>
            <div class="keys">
              <div class="keys-row"><span>{t('Andar pelo mapa')}</span><kbd>← ↑ → ↓</kbd></div>
              <div class="keys-row"><span>{t('Subir ou descer entre irmãos')}</span><kbd>{alt}↑ {alt}↓</kbd></div>
              <div class="keys-row"><span>{t('Ir para a raiz')}</span><kbd>Home</kbd></div>
              <div class="keys-row"><span>{t('Recolher ou expandir')}</span><kbd>{mod}/</kbd></div>
              <div class="keys-row"><span>{t('Excluir')}</span><kbd>Delete</kbd></div>
              <div class="keys-row"><span>{t('Menu do nó')}</span><kbd>⇧F10</kbd></div>
            </div>
            <span class="keys-head">{t('Escolher vários')}</span>
            <div class="keys">
              <div class="keys-row"><span>{t('Escolher num retângulo')}</span><kbd>{prefs.dragBackground === 'select' ? t('arrastar o fundo') : t('⇧ + arrastar o fundo')}</kbd></div>
              <div class="keys-row"><span>{t('Somar ou tirar um tópico')}</span><kbd>{t('⇧ ou {mod} + clique', { mod: mod.replace('+', '') })}</kbd></div>
              <div class="keys-row"><span>{t('Escolher tudo')}</span><kbd>{mod}A</kbd></div>
              <div class="keys-row"><span>{t('Desfazer a escolha')}</span><kbd>Esc</kbd></div>
            </div>
            <span class="keys-head">{t('Ver')}</span>
            <div class="keys">
              <div class="keys-row"><span>{t('Mover o mapa')}</span><kbd>{prefs.dragBackground === 'select' ? t('Espaço + arrastar') : t('arrastar o fundo')}</kbd></div>
              <div class="keys-row"><span>{t('Ver o mapa inteiro')}</span><kbd>⇧1</kbd></div>
              <div class="keys-row"><span>{t('Enquadrar a seleção')}</span><kbd>⇧2</kbd></div>
              <div class="keys-row"><span>{t('Tamanho real (100%)')}</span><kbd>{mod}0</kbd></div>
              <div class="keys-row"><span>{t('Aproximar ou afastar')}</span><kbd>{mod}= {mod}-</kbd></div>
              <div class="keys-row"><span>{t('Buscar')}</span><kbd>{mod}F</kbd></div>
            </div>
            <span class="keys-head">{t('Arquivo')}</span>
            <div class="keys">
              <div class="keys-row"><span>{t('Novo mapa (aba nova)')}</span><kbd>{t('{a} ou {b}', { a: `${mod}N`, b: `${mod}T` })}</kbd></div>
              <div class="keys-row"><span>{t('Fechar a aba')}</span><kbd>{mod}W</kbd></div>
              <div class="keys-row"><span>{t('Próxima aba ou anterior')}</span><kbd>Ctrl+Tab Ctrl+⇧Tab</kbd></div>
              <div class="keys-row"><span>{t('Abrir')}</span><kbd>{mod}O</kbd></div>
              <div class="keys-row"><span>{t('Salvar')}</span><kbd>{mod}S</kbd></div>
              <div class="keys-row"><span>{t('Salvar como')}</span><kbd>{mod}⇧S</kbd></div>
              <div class="keys-row"><span>{t('Exportar')}</span><kbd>{mod}⇧E</kbd></div>
              <div class="keys-row"><span>{t('Desfazer')}</span><kbd>{mod}Z</kbd></div>
              <div class="keys-row"><span>{t('Refazer')}</span><kbd>{mod}⇧Z</kbd></div>
              <div class="keys-row"><span>{t('Configurações')}</span><kbd>{mod},</kbd></div>
            </div>
        </section>

        <p class="footnote">
          {isPreview ? t('Prévia no navegador · dados de exemplo') : `Nodo${version ? ` v${version}` : ''}`}
          {#if updates.info}
            · <button class="link" onclick={() => updates.install()} disabled={updates.phase === 'downloading'}>
              {updates.phase === 'downloading' ? t('Atualizando…') : t('Atualizar para a v{versao}', { versao: updates.info.version })}
            </button>
          {:else if hasHost && updates.supported}
            · <button class="link" onclick={checkNow} disabled={updates.phase === 'checking'}>
              {updates.phase === 'checking' ? t('Procurando…') : t('Procurar atualizações')}
            </button>
          {/if}
        </p>
      </div>
    </Dialog.Content>
  </Dialog.Portal>
</Dialog.Root>

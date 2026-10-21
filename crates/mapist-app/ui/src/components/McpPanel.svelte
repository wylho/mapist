<script lang="ts">
  import { Check, Copy } from '@lucide/svelte'
  // Página de MCP: mostra como ligar o Mapist a um agente e registra no
  // Claude Desktop com um clique, como faz o Ideario.
  import { onMount } from 'svelte'
  import { api, hasHost, type McpInfo } from '../lib/api'

  let info = $state<McpInfo | null>(null)
  let error = $state<string | null>(null)
  let copied = $state(false)
  let busy = $state(false)

  async function refresh() {
    if (!hasHost) return
    try {
      info = await api.mcpInfo()
      error = null
    } catch (e) {
      error = String(e)
    }
  }

  async function install() {
    busy = true
    try {
      info = await api.mcpInstall()
      error = null
    } catch (e) {
      error = String(e)
    } finally {
      busy = false
    }
  }

  async function uninstall() {
    busy = true
    try {
      info = await api.mcpUninstall()
      error = null
    } catch (e) {
      error = String(e)
    } finally {
      busy = false
    }
  }

  async function copyCommand() {
    if (!info) return
    try {
      await navigator.clipboard.writeText(info.claudeCode)
      copied = true
      setTimeout(() => (copied = false), 1600)
    } catch (e) {
      error = String(e)
    }
  }

  onMount(refresh)
</script>

<div class="card">
  <div class="card-body field">
    <span class="field-label">Servidor MCP</span>

    {#if !hasHost}
      <div class="banner">Disponível apenas no aplicativo.</div>
    {:else if !info}
      <div class="banner">Carregando…</div>
    {:else}
      <p class="hint" style="margin:0">
        O Mapist fala MCP pelo próprio executável. Um agente abre e edita mapas pelo mesmo
        núcleo do aplicativo, e cada mudança entra no mesmo histórico de desfazer.
      </p>

      <div class="field">
        <span class="field-label">Claude Desktop</span>
        {#if info.desktopInstalled}
          <div class="seg tight">
            <span class="seg-item" data-state="on" style="pointer-events:none">Registrado</span>
            <button class="seg-item" data-state="off" disabled={busy} onclick={uninstall}>
              Remover
            </button>
          </div>
        {:else}
          <button class="btn primary" disabled={busy || !info.desktopFound} onclick={install}>
            Registrar no Claude Desktop
          </button>
          {#if !info.desktopFound}
            <span class="field-hint">Claude Desktop não encontrado nesta máquina.</span>
          {/if}
        {/if}
      </div>

      <div class="field">
        <span class="field-label">
          Claude Code <span class="field-hint">cole no terminal</span>
        </span>
        <div class="cmd">
          <code>{info.claudeCode}</code>
          <button class="icon-btn sm" aria-label="Copiar comando" title="Copiar" onclick={copyCommand}>
            {#if copied}
              <Check size={14} strokeWidth={2.4} />
            {:else}
              <Copy size={14} />
            {/if}
          </button>
        </div>
      </div>

      {#if info.desktopConfig}
        <span class="field-hint" style="overflow-wrap:anywhere">{info.desktopConfig}</span>
      {/if}
    {/if}

    {#if error}
      <div class="banner" style="color:var(--late)">{error}</div>
    {/if}
  </div>
</div>

<style>
  .hint {
    font-size: 12.5px;
    color: var(--muted);
    line-height: 1.45;
  }
  .cmd {
    display: flex;
    align-items: center;
    gap: 6px;
    background: var(--raised);
    border: 1px solid var(--line);
    border-radius: 12px;
    padding: 6px 6px 6px 11px;
  }
  .cmd code {
    flex: 1;
    min-width: 0;
    font-family: var(--font-mono);
    font-size: 11.5px;
    overflow-x: auto;
    white-space: nowrap;
    -webkit-user-select: text;
    user-select: text;
  }
</style>

<script lang="ts">
  // Tela inicial (pedido do usuário; opção A, lista com 8): sem mapa aberto, Novo e Abrir no meio do mapa e os últimos
  // mapas abertos ou salvos embaixo. Aparece ao abrir o app sem arquivo e ao fechar a última aba.
  import { FilePlus2, FileText, FolderOpen } from '@lucide/svelte'
  import { t } from '@wylho/atrio'
  import { doc } from '../lib/document.svelte'
  import { recents, recentFolder, recentName, recentWhen } from '../lib/recents.svelte'
  import { ui } from '../lib/ui.svelte'

  const mod = typeof navigator !== 'undefined' && /Mac|iPhone|iPad/.test(navigator.platform) ? '⌘' : 'Ctrl+'

  async function openRecent(path: string) {
    await doc.openPath(path)
    // Apagado ou movido: sai da lista, para não ficar oferecendo um arquivo que não abre.
    if (doc.error && !doc.hasDocument) {
      recents.remove(path)
      ui.say(t('“{nome}” não foi encontrado e saiu dos recentes.', { nome: recentName(path) }), 5200)
    }
  }
</script>

<div class="start">
  <div class="start-box">
    <h1>{t('Comece um mapa')}</h1>
    <div class="start-actions">
      <button class="start-btn primary" onclick={() => doc.newMap()}><FilePlus2 size={20} /> <span>{t('Novo mapa')}</span> <kbd>{mod}N</kbd></button>
      <button class="start-btn" onclick={() => doc.openDialog()}><FolderOpen size={20} /> <span>{t('Abrir…')}</span> <kbd>{mod}O</kbd></button>
    </div>

    <h2>{t('Recentes')}</h2>
    {#if recents.list.length}
      <ul class="start-list">
        {#each recents.list as r (r.path)}
          <li>
            <button title={r.path} onclick={() => openRecent(r.path)}>
              <FileText size={18} />
              <span class="name">{recentName(r.path)}</span>
              <span class="dir">{recentFolder(r.path)}</span>
              <span class="when">{recentWhen(r.at)}</span>
            </button>
          </li>
        {/each}
      </ul>
    {:else}
      <p class="start-empty">{t('Os mapas que você abrir ou salvar aparecem aqui.')}</p>
    {/if}
  </div>
</div>

<style>
  .start { position: absolute; inset: 0; display: grid; place-items: center; grid-template-columns: minmax(0, 1fr); background: var(--stage); z-index: 40; overflow: auto; padding: 24px 16px; }
  .start-box { min-width: 0; width: min(520px, 100%); display: flex; flex-direction: column; gap: 14px; }
  h1 { font-size: 26px; font-weight: 700; margin: 0 0 4px; color: var(--fg); text-align: center; text-wrap: balance; }
  h2 { font-size: 13px; font-weight: 600; color: var(--muted); margin: 14px 0 0; text-transform: uppercase; letter-spacing: .04em; }
  .start-actions { display: grid; grid-template-columns: minmax(0, 1fr) minmax(0, 1fr); gap: 12px; }
  .start-btn { display: flex; align-items: center; gap: 10px; height: 56px; padding: 0 18px; border-radius: 16px; background: var(--surface); border: 1px solid var(--line); color: var(--fg); font-weight: 600; font-size: 15px; box-shadow: var(--shadow); }
  .start-btn:hover { background: var(--raised); }
  .start-btn span { flex: 1; min-width: 0; text-align: left; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .start-btn.primary { background: var(--accent); color: var(--on-accent); border-color: transparent; }
  .start-btn.primary:hover { filter: brightness(1.06); }
  .start-btn kbd { font: 12px var(--font-mono); opacity: .7; }
  .start-btn:focus-visible, .start-list button:focus-visible { outline: 2px solid var(--accent); outline-offset: 2px; }
  .start-list { list-style: none; margin: 0; padding: 6px; background: var(--surface); border: 1px solid var(--line); border-radius: 16px; box-shadow: var(--shadow); }
  .start-list button { width: 100%; display: grid; grid-template-columns: 18px minmax(0, 1fr) auto; grid-template-rows: auto auto; column-gap: 12px; align-items: center; padding: 9px 12px; border-radius: 10px; text-align: left; color: var(--fg); }
  .start-list button:hover { background: var(--raised); }
  .start-list button :global(svg) { grid-row: span 2; color: var(--muted); }
  .name { font-weight: 600; font-size: 14px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .dir { grid-column: 2; font-size: 12px; color: var(--muted); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .when { grid-column: 3; grid-row: 1 / span 2; font-size: 12px; color: var(--muted); font-variant-numeric: tabular-nums; }
  .start-empty { margin: 0; color: var(--muted); font-size: 14px; }
  @media (max-width: 560px) { .start-btn kbd { display: none; } }
</style>

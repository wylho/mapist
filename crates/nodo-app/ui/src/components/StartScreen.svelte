<script lang="ts">
  // PRÉVIA DE OPÇÕES (provisório): tela inicial com Novo, Abrir e os mapas recentes, no meio da área do mapa.
  import { FilePlus2, FolderOpen, FileText } from '@lucide/svelte'
  import { t } from '@wylho/atrio'

  let { variant = 'a' }: { variant?: string } = $props()
  const mod = typeof navigator !== 'undefined' && /Mac|iPhone|iPad/.test(navigator.platform) ? '⌘' : 'Ctrl+'

  // Dados fictícios só para a prévia.
  const recents = [
    { name: 'Viagem em grupo', dir: '~/Documentos/Planos', when: 'Hoje, 14:20', colors: ['#6aaef5', '#b28ce6', '#7fc07f'] },
    { name: 'Roteiro do podcast', dir: '~/Documentos', when: 'Ontem, 09:05', colors: ['#f0a35e', '#6aaef5', '#e7c24a'] },
    { name: 'Estudos de química', dir: '~/Escola', when: 'Seg., 18:40', colors: ['#7fc07f', '#e78a9a', '#6aaef5'] },
    { name: 'Lançamento do app', dir: '~/Trabalho', when: '12 de out.', colors: ['#b28ce6', '#e7c24a', '#7fc07f'] },
    { name: 'Horta da varanda', dir: '~/Documentos', when: '3 de out.', colors: ['#7fc07f', '#f0a35e', '#e78a9a'] },
    { name: 'Ideias para o livro', dir: '~/Escrita', when: '28 de set.', colors: ['#e78a9a', '#6aaef5', '#b28ce6'] },
    { name: 'Mudança de casa', dir: '~/Documentos', when: '15 de set.', colors: ['#e7c24a', '#7fc07f', '#6aaef5'] },
    { name: 'Curso de fotografia', dir: '~/Cursos', when: '2 de set.', colors: ['#6aaef5', '#e7c24a', '#f0a35e'] },
  ]
</script>

<div class="start" data-variant={variant}>
  <div class="start-box" class:wide={variant === 'b'}>
    <h1>{t('Comece um mapa')}</h1>
    <div class="start-actions">
      <button class="start-btn primary"><FilePlus2 size={20} /> <span>{t('Novo mapa')}</span> <kbd>{mod}N</kbd></button>
      <button class="start-btn"><FolderOpen size={20} /> <span>{t('Abrir…')}</span> <kbd>{mod}O</kbd></button>
    </div>

    <h2>{t('Recentes')}</h2>
    {#if variant === 'b'}
      <div class="start-grid">
        {#each recents.slice(0, 6) as r}
          <button class="start-card">
            <svg viewBox="0 0 160 90" class="thumb" aria-hidden="true">
              <rect x="58" y="38" width="44" height="14" rx="7" fill="var(--accent)" />
              {#each r.colors as c, i}
                <path d="M102 45 C 115 45, 112 {20 + i * 25}, 122 {20 + i * 25}" stroke={c} stroke-width="2" fill="none" />
                <rect x="122" y={14 + i * 25} width="30" height="12" rx="6" fill={c} />
              {/each}
              <path d="M58 45 C 45 45, 48 30, 38 30" stroke={r.colors[1]} stroke-width="2" fill="none" />
              <rect x="8" y="24" width="30" height="12" rx="6" fill={r.colors[1]} />
              <path d="M58 45 C 45 45, 48 62, 38 62" stroke={r.colors[2]} stroke-width="2" fill="none" />
              <rect x="8" y="56" width="30" height="12" rx="6" fill={r.colors[2]} />
            </svg>
            <span class="name">{r.name}</span>
            <span class="when">{r.when}</span>
          </button>
        {/each}
      </div>
    {:else}
      <ul class="start-list">
        {#each recents as r}
          <li>
            <button>
              <FileText size={18} />
              <span class="name">{r.name}</span>
              <span class="dir">{r.dir}</span>
              <span class="when">{r.when}</span>
            </button>
          </li>
        {/each}
      </ul>
    {/if}
  </div>
</div>

<style>
  .start { position: absolute; inset: 0; display: grid; place-items: center; grid-template-columns: minmax(0, 1fr); background: var(--stage); z-index: 40; overflow: auto; padding: 24px 16px; }
  .start-box { min-width: 0; width: min(520px, 100%); display: flex; flex-direction: column; gap: 14px; }
  .start-box.wide { width: min(640px, 100%); }
  h1 { font-size: 26px; font-weight: 700; margin: 0 0 4px; color: var(--fg); text-align: center; }
  h2 { font-size: 13px; font-weight: 600; color: var(--muted); margin: 14px 0 0; text-transform: uppercase; letter-spacing: .04em; }
  .start-actions { display: grid; grid-template-columns: 1fr 1fr; gap: 12px; }
  .start-btn { display: flex; align-items: center; gap: 10px; height: 56px; padding: 0 18px; border-radius: 16px; background: var(--surface); border: 1px solid var(--line); color: var(--fg); font-weight: 600; font-size: 15px; box-shadow: var(--shadow); }
  .start-btn span { flex: 1; text-align: left; }
  .start-btn.primary { background: var(--accent); color: var(--on-accent); border-color: transparent; }
  .start-btn kbd { font: 12px var(--font-mono); opacity: .7; }
  .start-list { list-style: none; margin: 0; padding: 6px; background: var(--surface); border: 1px solid var(--line); border-radius: 16px; box-shadow: var(--shadow); }
  .start-list button { width: 100%; display: grid; grid-template-columns: 18px 1fr auto; grid-template-rows: auto auto; column-gap: 12px; align-items: center; padding: 9px 12px; border-radius: 10px; text-align: left; color: var(--fg); }
  .start-list button:hover { background: var(--raised); }
  .start-list :global(svg) { grid-row: span 2; color: var(--muted); }
  .start-list .name { font-weight: 600; font-size: 14px; }
  .start-list .dir { grid-column: 2; font-size: 12px; color: var(--muted); }
  .start-list .when { grid-column: 3; grid-row: 1 / span 2; font-size: 12px; color: var(--muted); }
  .start-grid { display: grid; grid-template-columns: repeat(3, minmax(0, 1fr)); gap: 12px; }
  .start-card { display: flex; flex-direction: column; gap: 4px; padding: 8px 8px 10px; background: var(--surface); border: 1px solid var(--line); border-radius: 14px; text-align: left; box-shadow: var(--shadow); color: var(--fg); }
  .start-card:hover { border-color: var(--accent); }
  .thumb { display: block; flex: none; width: 100%; height: auto; aspect-ratio: 16 / 9; background: var(--raised); border-radius: 9px; margin-bottom: 4px; }
  .start-card .name { font-weight: 600; font-size: 13.5px; padding: 0 4px; }
  .start-card .when { font-size: 12px; color: var(--muted); padding: 0 4px; }
  @media (max-width: 560px) { .start-grid { grid-template-columns: repeat(2, minmax(0, 1fr)); } .start-btn kbd { display: none; } .start-actions { grid-template-columns: minmax(0, 1fr) minmax(0, 1fr); } .start-list .dir { display: none; } }
</style>

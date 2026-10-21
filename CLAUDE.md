# Mapist — app de mapas mentais local-first (Tauri 2 + Rust + Svelte 5)

App de mapas mentais multiplataforma, derivado do [Minder](https://github.com/phase1geo/Minder), com servidor MCP embutido
para que um agente leia e edite os mapas pelo mesmo núcleo do aplicativo.
**Especificação completa: `docs/SPEC.md`. Leia antes de qualquer tarefa.** O protótipo de design está em `prototype/`.

## Regras do projeto
- **Local-first**: o mapa vive num arquivo no disco do usuário. Nada de servidor, conta ou rede para abrir, editar ou salvar.
- **O arquivo é do usuário**: formato `.minder` (XML do Minder 2.0.9), legível sem o app e compatível com o Minder original.
  Ida e volta sem perda nos campos que o Mapist modela.
- **Um núcleo, duas portas**: o aplicativo e o servidor MCP usam o mesmo `mapist-core`. Um agente editando por MCP
  produz exatamente o mesmo resultado que uma pessoa editando na janela, inclusive no histórico de desfazer.
- **Um binário, dois modos**: `mapist` abre a janela; `mapist --mcp` fala o protocolo. O instalador leva um arquivo só.
- **A geometria do mapa não depende do zoom.** Medição e desenho acontecem em unidades de mundo; o zoom entra uma única
  vez, por `ctx.setTransform`. Misturar os dois espaços trunca texto — foi o defeito mais caro já corrigido aqui.
- **Texto é medido no frontend**, com métricas de fonte reais, e enviado ao núcleo por `set_sizes`. O Rust só posiciona:
  duplicar a pilha de fontes lá daria medidas divergentes do que a tela desenha.
- Fidelidade visual ao **Ideario** (`crates/mapist-app/ui/src/app.css` e `src/lib/palettes.ts` são a referência de tokens).
  Quando o Ideario mudar uma cor, replicar aqui em vez de inventar.
- UI em **português do Brasil**. Código e identificadores em inglês.
- **Dados de exemplo sempre fictícios** (mock, testes, docs, prints do site/README): nunca nada da vida real do
  usuário. O repositório é público. Antes de publicar um print, olhar o print inteiro.
- **Git:** commit e push ao terminar cada tarefa (o ambiente é temporário). Lançar versão só quando o usuário pedir.
- **Nada de controles que ainda não funcionam**; só mostrar o que tem motivo para aparecer.
- Trabalhar por fases (SPEC §10). Antes de começar uma fase, conferir as decisões pendentes (SPEC §11) e perguntar o que
  estiver em aberto. Ao fim de cada fase: compilar, rodar e verificar o critério "Pronto quando".
- **Mudança visual: validar no preview antes de compilar.** O preview (`pnpm dev` em `crates/mapist-app/ui`, com
  `?preview`) roda a interface real no navegador sobre um backend simulado, com HMR. Compilar o Tauri leva minutos e não
  deixa inspecionar nada; o preview mostra em segundos. Conferir nos temas claro e escuro, e na largura de celular.
  Antes de aplicar, mostrar prévias (prints, opções A/B/C); conferir o print da interface de verdade, não só o código.
- **Verificação antes de dizer "pronto":** `cargo fmt --all --check`, `cargo clippy --all-targets -- -D warnings`,
  `cargo test --workspace`, `pnpm --dir crates/mapist-app/ui run check`, `pnpm --dir crates/mapist-app/ui run test`,
  e para mudança que afeta o app: `cargo tauri build` (o `beforeBuildCommand` gera o frontend; `cargo build` sozinho
  **não** embute a interface e produz uma janela vazia).
- Ao terminar cada tarefa, atualizar a seção "Estado atual" deste arquivo (o que mudou e em quais arquivos).

## Decisões tomadas
- **D1:** frontend em **Svelte 5 + Vite + TypeScript**, mesma stack e versões do Ideario (svelte 5.57, vite 8.3, TS 5.9).
  HTML/JS puro foi descartado: sem componentes, o porte do design system viraria duplicação.
- **D2:** desktop primeiro (macOS, Linux, Windows). O Minder original é GTK4 e não roda bem no macOS, que é o alvo principal.
- **D3:** nome **Mapist**. Binário `mapist`; crates `mapist-core`, `mapist-mcp`, `mapist-app`.
- **D4:** licença **GPL-3.0-or-later**, herdada do Minder (é obra derivada; o copyleft vale).
- **D5:** formato **`.minder`** (XML do Minder 2.0.9), não um formato próprio — manter os mapas existentes abríveis vale
  mais que a liberdade de inventar. Campos novos entram como atributos ignorados pelo Minder original.
- **D6:** o servidor MCP é o **próprio executável** com `--mcp` (padrão copiado do Ideario), não um segundo binário.
  O `mapist-mcp` continua existindo como binário enxuto para uso sem interface.
- **D7:** **três paletas nomeadas + tema Sistema** (Ideario, Papel, Grafite, Floresta; Sistema imita GNOME, KDE, Windows,
  macOS), cada uma clara e escura. Os tokens são copiados de `ideario/src/lib/palettes.ts`; o `fitAccent` garante que
  qualquer destaque do sistema continue legível.
- **D8:** **medição de texto no frontend** (`set_sizes`), não no Rust. Uma crate de fontes no núcleo duplicaria a pilha e
  divergiria do que a tela desenha.
- **D9:** preferências (tema, zoom, painel, busca) em `localStorage`. O documento nunca passa por lá — ele é do Rust.

## Estado atual
- **Fase 1 (núcleo) e Fase 2 (janela) concluídas.** O app abre, desenha e edita mapas; o MCP responde no mesmo binário.
- `mapist-core` (`crates/mapist-core/src/`): modelo (`model.rs` — `Node`, `MindMap`, `Style`, enums), I/O do `.minder`
  (`document.rs`, parser manual com quick-xml), layout (`layout.rs` — `layout` mede e posiciona, `reposition` só
  posiciona quando as medidas vieram do frontend), histórico (`undo.rs` — `Action` + `History` por instantâneos).
  15 testes. Verificado contra um arquivo real do Minder: 498 nós, profundidade 4, ida e volta sem perda.
- `mapist-mcp` (`crates/mapist-mcp/src/`): servidor JSON-RPC sobre stdio (`lib.rs`, `serve()`), 16 ferramentas
  (`tools.rs`), registro no cliente (`setup.rs` — recusa JSON inválido e escreve por arquivo temporário + rename).
  25 testes. Negocia a versão do protocolo e envia `instructions` no `initialize`.
- `mapist-app` (`crates/mapist-app/`): janela Tauri com 18 comandos (`src/lib.rs`), gerenciador de diálogo, estado de
  janela e abertura de links. Interface em `ui/src/`: `App.svelte`, `components/` (Canvas, Toolbar, Inspector,
  ThemeSheet, McpPanel) e `lib/` (palettes, theme, api, document, measure, render, font, prefs). 5 testes Rust + 24 do
  frontend (Vitest).
- **46 testes Rust + 24 do frontend = 70.** `cargo clippy -D warnings` e `svelte-check` limpos.
- **Preview** (`ui/src/lib/preview.ts`): backend simulado em memória que espelha o núcleo, sobre um mapa de exemplo.
  Ativa com `?preview` e é o caminho para validar qualquer mudança visual antes de compilar.
- **Pendente:** os workflows de CI (`build.yml`, `release.yml`) estão escritos e validados, mas não versionados — o
  token OAuth do `gh` não tem o escopo `workflow`. Cópias em `/tmp/mapist-ci/`.
- **Próximo:** Fase 3 (importação e exportação: Markdown, OPML, FreeMind, Mermaid, SVG, PNG). Antes dela, revisar a
  lista de lacunas conhecidas no README (diálogo nativo de arquivo, medição no render, testes E2E nativos).

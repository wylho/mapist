# Nodo — app de mapas mentais local-first (Tauri 2 + Rust + Svelte 5)

App de mapas mentais multiplataforma, derivado do [Minder](https://github.com/phase1geo/Minder), com servidor MCP embutido
para que um agente leia e edite os mapas pelo mesmo núcleo do aplicativo.
**Especificação completa: `docs/SPEC.md`. Leia antes de qualquer tarefa.** O protótipo de design está em `prototype/`.

## Regras do projeto
- **Local-first**: o mapa vive num arquivo no disco do usuário. Nada de servidor, conta ou rede para abrir, editar ou salvar.
- **O arquivo é do usuário**: formato `.minder` (XML do Minder 2.0.9), legível sem o app e compatível com o Minder original.
  Ida e volta sem perda nos campos que o Nodo modela.
- **Um núcleo, duas portas**: o aplicativo e o servidor MCP usam o mesmo `nodo-core`. Um agente editando por MCP
  produz exatamente o mesmo resultado que uma pessoa editando na janela, inclusive no histórico de desfazer.
- **Um binário, dois modos**: `nodo` abre a janela; `nodo --mcp` fala o protocolo. O instalador leva um arquivo só.
- **A geometria do mapa não depende do zoom.** Medição e desenho acontecem em unidades de mundo; o zoom entra uma única
  vez, por `ctx.setTransform`. Misturar os dois espaços trunca texto — foi o defeito mais caro já corrigido aqui.
- **Texto é medido no frontend**, com métricas de fonte reais, e enviado ao núcleo por `set_sizes`. O Rust só posiciona:
  duplicar a pilha de fontes lá daria medidas divergentes do que a tela desenha.
- Fidelidade visual ao **Ideario** (`crates/nodo-app/ui/src/app.css` e `src/lib/palettes.ts` são a referência de tokens).
  Quando o Ideario mudar uma cor, replicar aqui em vez de inventar.
- UI em **português do Brasil**. Código e identificadores em inglês.
- **Dados de exemplo sempre fictícios** (mock, testes, docs, prints do site/README): nunca nada da vida real do
  usuário. O repositório é público. Antes de publicar um print, olhar o print inteiro.
- **Git:** commit e push ao terminar cada tarefa (o ambiente é temporário). Lançar versão só quando o usuário pedir.
- **Nada de controles que ainda não funcionam**; só mostrar o que tem motivo para aparecer.
- Trabalhar por fases (SPEC §10). Antes de começar uma fase, conferir as decisões pendentes (SPEC §11) e perguntar o que
  estiver em aberto. Ao fim de cada fase: compilar, rodar e verificar o critério "Pronto quando".
- **Mudança visual: validar no preview antes de compilar.** O preview (`pnpm dev` em `crates/nodo-app/ui`, com
  `?preview`) roda a interface real no navegador sobre um backend simulado, com HMR. Compilar o Tauri leva minutos e não
  deixa inspecionar nada; o preview mostra em segundos. Conferir nos temas claro e escuro, e na largura de celular.
  Antes de aplicar, mostrar prévias (prints, opções A/B/C); conferir o print da interface de verdade, não só o código.
- **Verificação antes de dizer "pronto":** `cargo fmt --all --check`, `cargo clippy --all-targets -- -D warnings`,
  `cargo test --workspace`, `pnpm --dir crates/nodo-app/ui run check`, `pnpm --dir crates/nodo-app/ui run test`,
  e para mudança que afeta o app: `crates/nodo-app/ui/node_modules/.bin/tauri build` na raiz (o `beforeBuildCommand`
  gera o frontend; `cargo build` sozinho **não** embute a interface e produz uma janela vazia). No Linux sem tela:
  `xvfb-run -a target/release/nodo arquivo.minder` e um print com `import -window root`.
- **Skills do usuário para todas as fases** (em `.claude/skills/`): `rust-testing` (teste antes do código; proptest
  para propriedades; provar que o teste pega o erro quebrando o código de propósito; clippy sem avisos) e
  `rust-patterns` (nada de `unwrap` em produção; `pub` só no necessário; erros com tipo próprio). `sync-site` mantém
  site e README juntos.
- Ao terminar cada tarefa, atualizar a seção "Estado atual" deste arquivo (o que mudou e em quais arquivos).

## Decisões tomadas
- **D1:** frontend em **Svelte 5 + Vite + TypeScript**, mesma stack e versões do Ideario (svelte 5.57, vite 8.3, TS 5.9).
  HTML/JS puro foi descartado: sem componentes, o porte do design system viraria duplicação.
- **D2:** desktop primeiro (macOS, Linux, Windows). O Minder original é GTK4 e não roda bem no macOS, que é o alvo principal.
- **D3:** nome **Nodo**. Binário `nodo`; crates `nodo-core`, `nodo-mcp`, `nodo-app`.
- **D4:** licença **GPL-3.0-or-later**, herdada do Minder (é obra derivada; o copyleft vale).
- **D5:** formato **`.minder`** (XML do Minder 2.0.9), não um formato próprio — manter os mapas existentes abríveis vale
  mais que a liberdade de inventar. Campos novos entram como atributos ignorados pelo Minder original.
- **D6:** o servidor MCP é o **próprio executável** com `--mcp` (padrão copiado do Ideario), não um segundo binário.
  O `nodo-mcp` continua existindo como binário enxuto para uso sem interface.
- **D7:** **paletas nomeadas + tema Sistema** (Nodo, Papel, Grafite, Floresta; a Nodo é a paleta-base do Ideario
  com o nome do app, pedido do usuário; Sistema imita GNOME, KDE, Windows,
  macOS), cada uma clara e escura. Os tokens são copiados de `ideario/src/lib/palettes.ts`; o `fitAccent` garante que
  qualquer destaque do sistema continue legível.
- **D8:** **medição de texto no frontend** (`set_sizes`), não no Rust. Uma crate de fontes no núcleo duplicaria a pilha e
  divergiria do que a tela desenha.
- **D9:** preferências (tema, zoom, painel, busca, traçado das linhas) em `localStorage`. O documento nunca passa por
  lá — ele é do Rust.
- **D10:** **um `Editor` no núcleo** (`nodo-core/src/editor.rs`) é o dono do mapa, do histórico, do arquivo e do
  estado salvo; o app e o MCP só o chamam. Motivo: cada porta guardava uma cópia do mapa ao lado do histórico, e
  recolher, mudar de lado e medidas se perdiam na edição seguinte.
- **D11:** **design system do Ideario por cópia literal** (`ui/src/ideario.css` = `ideario/src/app.css`; `map.css` só
  com o que é de mapa) e a **estrutura de UX do Ideario como padrão dos nossos apps**: TopBar, folha de
  Configurações, toast, ConfirmDialog (bits-ui), ícones `@lucide/svelte`. Pedido do usuário.
- **D12:** **ícone "raiz e ramos"** (Figma `Windows-11-UI-Kit`, Page 20, escolha do usuário: verde, azul, amarelo, na
  linguagem do ícone do Ideario). Fontes em `crates/nodo-app/icons/source/` (placa arredondada, quadrado inteiro,
  grade do macOS e Linux sem placa a 1,12×). Regenerar: `tauri icon icons/source/nodo.svg -o icons`, o `.icns` de
  `macos.svg` e os quatro PNG do Linux de `linux.svg` (ver o commit que os gerou).
- **D13:** **site em nodoapp.pages.dev** (`site/`, Cloudflare Pages sem build), mesma estrutura e `style.css` do site
  do Ideario; README com o mesmo conteúdo (skill `sync-site`).

- **D14:** **edição como no XMind** (pedido do usuário: "não sei se já posso digitar"): sempre há um nó selecionado,
  digitar escreve nele, Espaço/F2 editam, Tab/Enter criam já em edição, recolher em ⌘/; dica de teclado no canto.
- **D15:** **temas de mapa no arquivo**, como no XMind (esqueleto + cores por ramo), separados da paleta de interface
  (que é do aparelho). Atributo `nodo-style` no `<theme>`, que o Minder ignora.
- **D16:** **barra de título do macOS como a do Ideario**: janela criada no `setup` com `TitleBarStyle::Overlay`,
  botões em (18, 33) dentro da barra de 64 px, espaço medido por `mac_lights_width`. Sem atualização automática por
  enquanto (decisão do usuário).

## Estado atual
- **Fases 1 e 2 concluídas. Versão 0.1.0 lançada** (Release `v0.1.0`: `.dmg` universal, `-setup.exe`/`.msi`,
  `.AppImage`/`.deb`/`.rpm`), sem atualização automática (decisão do usuário). O site depende de reconectar o Git no
  projeto do Cloudflare Pages (o projeto perdeu o acesso ao repositório e só tem a publicação de antes de `site/`).
- `nodo-core`: modelo (`model.rs`), `.minder` (`document.rs`), layout (`layout.rs`: faixa por sub-árvore, lado de cada
  ramo, descendentes de nó recolhido sem posição), histórico (`undo.rs`: `Action` inclui `SetSide`, `ToggleFold`,
  `Relayout`), **`Editor`** (`editor.rs`: histórico + arquivo + revisões; sujo segue o desfazer até o salvo; salvar é
  atômico; `changed_on_disk` para ver o que o agente salvou). Testes em `editor.rs`, `lib.rs` e
  `tests/layout_props.rs` (proptest).
- `nodo-mcp`: `Session` = `Option<Editor>`; 17 ferramentas (`toggle_fold` nova; `set_side`/`relayout` passam pelo
  histórico); `setup.rs` registra no Claude Desktop.
- `nodo-app`: 21 comandos em `src/lib.rs` sobre o `Editor` (`set_side`, `reload_map`, `quit_app` novos); snapshot
  esconde filhos de nó recolhido e diz se o arquivo mudou no disco (`stale`); fechar/Cmd+Q com algo não salvo pergunta
  na interface (evento `close-requested`); `.minder` associado no instalador e aberto por argumento ou pelo evento do
  macOS (`open-file`). Ícones de todas as plataformas em `icons/`, `bundle.targets = all`.
- Interface (`ui/src/`): `App.svelte` (casca do Ideario, atalhos ⌘N/⌘O/⌘S/⌘⇧S/⌘F/⌘,/⌘0), `components/` (TopBar,
  Canvas, Inspector com lado do ramo, SettingsSheet, ConfirmDialog, Picker) e `lib/` (api, document, ui, prefs,
  palettes, theme, render com `traceBranch` curvas/em ângulo, measure, font, preview). `ideario.css` + `map.css`.
- **Preview** (`ui/src/lib/preview.ts`): espelha o núcleo (mesmo layout, recolhidos escondidos); `?preview`, e
  `?select=Texto` abre com o nó selecionado. Mapa de exemplo "Lançamento do app", inventado.
- **Testes:** 64 Rust + 36 do frontend. clippy e svelte-check limpos. Build real verificado no Linux (`.deb`,
  `.AppImage`; a janela abre um `.minder` passado como argumento) e `nodo --mcp` testado de ponta a ponta.
- **CI/Release:** `.github/workflows/build.yml` (verificação + instaladores dos três sistemas nos Artifacts) e
  `release.yml` (rascunho → publicar); `scripts/check-version.sh` e `scripts/bump-version.sh`; guia em
  `docs/RELEASE.md`.
- **Site:** `site/` (index, privacidade, prints de `ui/scripts/site-shots.mjs`). Precisa ligar o projeto no Cloudflare
  Pages (pasta `site`, sem build) para ir ao ar.
- **Edição XMind e temas de mapa** (`lib/maptheme.ts`, `components/MapThemeSheet.svelte`, `Action::SetMapStyle`,
  `set_map_theme` no MCP, `set_map_style` no app); barra do macOS integrada (`setup` + `mac_lights_width` em
  `src/lib.rs`, `main.ts`, `map.css`).
- **Próximo:** a Fase 3 (importação e exportação). Atualização automática fica para depois (decisão do usuário).

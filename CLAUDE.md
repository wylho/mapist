# Nodo — app de mapas mentais local-first (Tauri 2 + Rust + Svelte 5)

App de mapas mentais multiplataforma, moderno, com servidor MCP embutido para que um agente leia e edite os mapas pelo
mesmo núcleo do aplicativo. Começou inspirado no [Minder](https://github.com/phase1geo/Minder); hoje é um app próprio
(D27): o Minder não é referência nem restrição.
**Especificação completa: `docs/SPEC.md`. Leia antes de qualquer tarefa.** O protótipo de design está em `prototype/`.

## Regras do projeto
- **O Atrio é o hub (regra do usuário, vale para toda tarefa):** tudo o que o Ideario e o Nodo têm em comum mora
  **só** no Atrio (`wylho/atrio`: componentes, CSS, regras do card, formatos, e no Rust a crate `atrio`), e os apps
  só usam. O card é **um só**: não parecido, **exatamente o mesmo** nos dois programas (mesma prévia, mesmas regras,
  mesmo visual). Antes de criar ou corrigir algo num app, perguntar: isto existe ou deveria existir no outro? Se sim,
  a mudança se faz no Atrio, sobe a tag e os **dois** apps sobem juntos (nunca deixar um app numa tag velha). Mesmo
  quando o usuário fala de um app só, se o assunto é peça do Atrio, o caminho é o Atrio. Plano em
  `atrio/docs/UNIFICACAO.md`.
- **Local-first**: o mapa vive num arquivo no disco do usuário. Nada de servidor, conta ou rede para abrir, editar ou salvar.
- **O arquivo é do usuário**: texto legível sem o app, gravado de forma atômica, ida e volta sem perda. É o `.nodo`
  (JSON, D28); compatibilidade com o Minder original **não** é requisito (D27).
- **Um núcleo, duas portas**: o aplicativo e o servidor MCP usam o mesmo `nodo-core`. Um agente editando por MCP
  produz exatamente o mesmo resultado que uma pessoa editando na janela, inclusive no histórico de desfazer.
- **Um binário, dois modos**: `nodo` abre a janela; `nodo --mcp` fala o protocolo. O instalador leva um arquivo só.
- **A geometria do mapa não depende do zoom.** Medição e desenho acontecem em unidades de mundo; o zoom entra uma única
  vez, por `ctx.setTransform`. Misturar os dois espaços trunca texto — foi o defeito mais caro já corrigido aqui.
- **Texto é medido no frontend**, com métricas de fonte reais, e enviado ao núcleo por `set_sizes`. O Rust só posiciona:
  duplicar a pilha de fontes lá daria medidas divergentes do que a tela desenha.
- Fidelidade visual ao **Ideario**: os tokens e o CSS vêm do Atrio (`@wylho/atrio/atrio.css`, D26); `src/lib/palettes.ts`
  segue as paletas do Ideario. Mudança de visual compartilhado se faz no Atrio, e aqui sobe a tag.
- UI em **português do Brasil**. Código e identificadores em inglês.
- **Dados de exemplo sempre fictícios** (mock, testes, docs, prints do site/README): nunca nada da vida real do
  usuário. O repositório é público. Antes de publicar um print, olhar o print inteiro.
- **Git (combinado com o usuário, como no Ideario):** commit e push ao terminar cada tarefa, no branch de trabalho **e
  no `main`** (o ambiente é temporário). O `main` é o branch de produção do Cloudflare Pages: push com mudança em
  `site/` publica o site em nodo-1c7.pages.dev. O push não chega a quem usa o app: só o workflow Release lança versão,
  e só quando o usuário pedir.
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
- **D5:** (substituída pela D28) o formato era o `.minder` do Minder 2.0.9.
- **D6:** o servidor MCP é o **próprio executável** com `--mcp` (padrão copiado do Ideario), não um segundo binário.
  O `nodo-mcp` continua existindo como binário enxuto para uso sem interface.
- **D7:** **paletas nomeadas + tema Sistema**, hoje **do Atrio** (Atrio, Papel, Grafite, Floresta; a Atrio é a
  paleta-base dos dois apps, pedido do usuário: o tema é do Atrio; a escolha gravada `nodo`/`ideario`/`mapist` vira
  `atrio`; Sistema imita GNOME, KDE, Windows,
  macOS), cada uma clara e escura. Os tokens vêm do Atrio (`src/lib/palettes.ts` só reexporta e monta o tema Sistema); o `fitAccent` garante que
  qualquer destaque do sistema continue legível.
- **D8:** **medição de texto no frontend** (`set_sizes`), não no Rust. Uma crate de fontes no núcleo duplicaria a pilha e
  divergiria do que a tela desenha.
- **D9:** preferências (tema, zoom, painel, busca, traçado das linhas) em `localStorage`. O documento nunca passa por
  lá — ele é do Rust.
- **D10:** **um `Editor` no núcleo** (`nodo-core/src/editor.rs`) é o dono do mapa, do histórico, do arquivo e do
  estado salvo; o app e o MCP só o chamam. Motivo: cada porta guardava uma cópia do mapa ao lado do histórico, e
  recolher, mudar de lado e medidas se perdiam na edição seguinte.
- **D11:** **design system do Ideario, hoje pelo Atrio** (D26; antes era cópia literal em `ui/src/ideario.css`;
  `map.css` só com o que é de mapa) e a **estrutura de UX do Ideario como padrão dos nossos apps**: TopBar, folha de
  Configurações, toast, ConfirmDialog (bits-ui), ícones `@lucide/svelte`. Pedido do usuário.
- **D12:** **ícone "raiz e ramos"** (Figma `Windows-11-UI-Kit`, Page 20, escolha do usuário: verde, azul, amarelo, na
  linguagem do ícone do Ideario). Fontes em `crates/nodo-app/icons/source/` (placa arredondada, quadrado inteiro,
  grade do macOS e Linux sem placa a 1,12×). Regenerar: `tauri icon icons/source/nodo.svg -o icons`, o `.icns` de
  `macos.svg` e os quatro PNG do Linux de `linux.svg` (ver o commit que os gerou).
- **D13:** **site em nodo-1c7.pages.dev** (`site/`, Cloudflare Pages sem build), mesma estrutura e `style.css` do site
  do Ideario; README com o mesmo conteúdo (skill `sync-site`). Diagramação inspirada no affinity.studio (abertura em
  palco com pedaços da interface em HTML, retos, em volta do título (não prints), print grande, seções com selo e título grande, cartões em dupla, faixas alternadas,
  rodapé em colunas), com os tokens do Ideario e ícones do Lucide inline. O `style.css` é o mesmo nos dois sites:
  mudou num, copiar para o outro.

- **D14:** **edição como no XMind** (pedido do usuário: "não sei se já posso digitar"): sempre há um nó selecionado,
  digitar escreve nele, Espaço/F2 editam, Tab/Enter criam já em edição, recolher em ⌘/; dica de teclado no canto.
- **D15:** **temas de mapa no arquivo**, como no XMind (esqueleto + cores por ramo), separados da paleta de interface
  (que é do aparelho). Atributo `nodo-style` no `<theme>`, que o Minder ignora.
- **D16:** **barra de título do macOS como a do Ideario**: janela criada no `setup` com `TitleBarStyle::Overlay`,
  botões em (18, 33) dentro da barra de 64 px, espaço medido por `mac_lights_width`.

- **D17:** **nome Nodo** (antes Mapist, que ficava a uma letra de uma palavra ofensiva em inglês; escolha do usuário).
  Repositório `wylho/nodo`, binário `nodo`, crates `nodo-*`, identificador `io.github.wylho.nodo`, site
  `nodo-1c7.pages.dev` (os nomes `nodo`, `nodoapp` e `usenodo` do Pages já tinham dono). O que a 0.1.0 gravou como
  Mapist continua sendo lido: atributo `mapist-style`, preferências `mapist.*`, entrada "mapist" no Claude Desktop.
- **D19:** **atualização automática a partir da 0.2.0**, igual à do Ideario (`src/updates.rs`, `lib/updates.svelte.ts`,
  `components/UpdateNotice.svelte`; `latest.json` do último Release, assinatura com a chave dos segredos
  `TAURI_SIGNING_PRIVATE_KEY(_PASSWORD)`). macOS, Windows e AppImage; `.deb`/`.rpm` não. A chave privada fica guardada
  pelo usuário no Ideario dele; perder a chave é perder a atualização de quem já instalou.
- **D20:** **repositórios apt e dnf como os do Ideario**, com uma chave GPG só do Nodo (escolha do usuário): os
  índices assinados vão em cada Release e o site redireciona `/apt` e `/rpm` para o último. A chave privada fica com o
  usuário, no Ideario dele.
- **D21:** **atalhos de zoom como XMind/MindNode e Figma** (pesquisa do usuário pedida): ⌘0 = 100% (antes era
  ajustar), ⌘=/⌘- , ⇧1 tudo, ⇧2 seleção (pelo `e.code`, independente do teclado). ⌥↑/⌥↓ do XMind para reordenar.
- **D22:** **organizar automaticamente por estrutura** (pedido do usuário, pesquisa XMind/MindNode): mapa, lógico,
  organograma e árvore, mais espaçamento; ficam no arquivo e desfazem. Espinha de peixe, matriz e linha do tempo
  ficaram para depois.
- **D23:** **exportar em formatos abertos** (pedido do usuário): PNG, SVG, PDF para imprimir (cabe numa página,
  retrato/paisagem automático), HTML de um arquivo só, Markdown e OPML. O PDF é imagem em alta resolução (igual à
  tela) em vez de vetor, para não depender de fontes embutidas; o SVG é o vetor.
- **D24:** **importar Markdown e OPML** (pedido do usuário) pelo próprio Abrir, como mapa novo não salvo.
- **D25:** **nó com nota é o card do Ideario** (escolha do usuário, opção A: fundo na cor do ramo clara), em HTML
  com o CSS do Ideario por cima do canvas, para ser idêntico; a nota se edita num card flutuante, e há notas soltas
  no mapa, gravadas no arquivo.
- **D26:** **Atrio** (`wylho/atrio`, pacote `@wylho/atrio`, nome escolhido pelo usuário): a biblioteca de
  componentes do Ideario e do Nodo, por dependência git numa tag (`github:wylho/atrio#vX.Y.Z`). Tem o `atrio.css`, o
  `CardBody` (miolo do card de nota), a prévia de Markdown com as regras do Ideario e o **card na área de
  transferência** (HTML com `data-atrio-card` + Markdown). Mudança de visual compartilhado se faz lá, e aqui só sobe
  a tag. Licença MIT (o Nodo é GPL e o Ideario não).
- **D27:** **o Nodo não é mais preso ao Minder** (pedido do usuário: "esquece o Minder"; foi inspiração, não é fork).
  Funcionalidade nova não precisa caber no formato nem no comportamento do Minder. Ainda em aberto com o usuário: a
  licença (a GPL veio do Minder; trocar pede conferir que nenhum código dele foi copiado). O rodapé do site mantém a
  atribuição enquanto isso.
- **D28:** **formato próprio `.nodo`** (escolha do usuário): JSON legível e versionado (`format: "nodo"`, `version`),
  só com o que o Nodo modela, sem posições (o layout é recalculado ao abrir). `nodo_core::nodofile` (`to_nodo`,
  `from_nodo`; aceita versão até `VERSION` e ignora campos desconhecidos), `load`/`save` gravam `.nodo`. Um `.minder`
  abre como importação (como Markdown/OPML: sem caminho, salvar pergunta); salvar com outra extensão grava o `.nodo`
  ao lado e devolve o caminho. Associação de arquivo: `.nodo` (editor) e `.minder` (leitor).
- **D29:** **o tópico é o card do Ideario** (pedido do usuário: "estamos criando um ideário que organiza as coisas
  por nós"). No mapa, o card da lista do Ideario (`MapCard` = `CardBody` + `CardPills` do Atrio); duplo clique ou
  "Abrir card" abre o mesmo card do Ideario (`CardDialog` + `NoteEditor`: diálogo na janela larga, tela cheia na
  estreita, modo documento), com `features({ category: false, attachments: false, capture: false, links: false })`.
  A edição do conteúdo sai da barra lateral (fica só o que é do mapa). Campos do card no núcleo: `CardFields`.
- **D30:** **o `.nodo` é um zip** (opção B, escolha do usuário depois de medir: abrir e salvar um mapa de 20 MB
  leva milissegundos): `mapa.json` (o JSON da D28, legível ao descompactar) + `anexos/<hash8>-<nome>` sem
  compressão. Notas citam o anexo como o Ideario (`![nome](ideario://att/<hash>)`, `[📎 nome](…)`), pelo
  `atrio::markdown`. `nodo_core::attachments` (hash = 16 bytes do SHA-256, bytes lidos sob demanda) e
  `nodo_core::package` (gravação atômica; anexo que nenhuma nota usa sai do arquivo mas fica na memória para o
  desfazer). O `.nodo` em JSON puro da 0.3.0 continua abrindo.
- **D18:** **o canvas redesenha no máximo uma vez por quadro**, e o que o desenho lê não pode ser `$state` escrito no
  mesmo efeito. Motivo: um laço assim (`colors`) fazia centenas de redesenhos por tecla e deixava a edição travando.

## Estado atual
- **Versão 0.3.0 lançada** (22/10/2026, Release `v0.3.0`: formato `.nodo`, o tópico é o card do Ideario, status no
  card do mapa, sem painel lateral). Quem tem a 0.2.0 recebe pelo atualizador.
- **Fases 1 e 2 concluídas. Versão 0.2.0 lançada** (21/10/2026, Release `v0.2.0` com `latest.json` e `.sig` de todos os
  instaladores: a primeira com atualização automática). A 0.1.0 saiu como Mapist e teve o Release apagado
- Antes: versão 0.1.0 (Release `v0.1.0`: `.dmg` universal, `-setup.exe`/`.msi`,
  `.AppImage`/`.deb`/`.rpm`), sem atualizador: quem tem a 0.1.0 baixa a 0.2.0 à mão; dali em diante é automático (D19).
- `nodo-core`: modelo (`model.rs`), `.minder` (`document.rs`), layout (`layout.rs`: faixa por sub-árvore, lado de cada
  ramo, descendentes de nó recolhido sem posição), histórico (`undo.rs`: `Action` inclui `SetSide`, `ToggleFold`,
  `Relayout`), **`Editor`** (`editor.rs`: histórico + arquivo + revisões; sujo segue o desfazer até o salvo; salvar é
  atômico; `changed_on_disk` para ver o que o agente salvou). Testes em `editor.rs`, `lib.rs` e
  `tests/layout_props.rs` (proptest).
- `nodo-mcp`: `Session` = `Option<Editor>`; 24 ferramentas (`toggle_fold`, `set_map_theme`, `move_sibling`, `set_layout`, `export_map` e as notas soltas `add_free_note`/`set_free_note`/`remove_free_note`; `set_side`/`relayout` passam pelo
  histórico); `setup.rs` registra no Claude Desktop.
- `nodo-app`: 32 comandos (com `reorder_node`, `set_structure`, `set_spacing`, `relayout`, `export_text`, `write_export`) (com `update_supported`/`update_check`/`update_install`) em `src/lib.rs` sobre o `Editor` (`set_side`, `reload_map`, `quit_app` novos); snapshot
  esconde filhos de nó recolhido e diz se o arquivo mudou no disco (`stale`); fechar/Cmd+Q com algo não salvo pergunta
  na interface (evento `close-requested`); `.minder` associado no instalador e aberto por argumento ou pelo evento do
  macOS (`open-file`). Ícones de todas as plataformas em `icons/`, `bundle.targets = all`.
- Interface (`ui/src/`): `App.svelte` (casca do Ideario, atalhos ⌘N/⌘O/⌘S/⌘⇧S/⌘F/⌘,/⌘0), `components/` (TopBar,
  Canvas, SettingsSheet, ConfirmDialog, Picker; o painel lateral (Inspector) saiu na 0.3.0: tudo do tópico está no
  card e no clique direito) e `lib/` (api, document, ui, prefs,
  palettes, theme, render com `traceBranch` curvas/em ângulo, measure, font, preview). `atrio.css` (do Atrio) + `map.css`.
- **Preview** (`ui/src/lib/preview.ts`): espelha o núcleo (mesmo layout, recolhidos escondidos); `?preview`, e
  `?select=Texto` abre com o nó selecionado. Mapa de exemplo "Lançamento do app", inventado.
- **Testes:** 107 Rust + 49 do frontend. clippy e svelte-check limpos. Build real verificado no Linux (`.deb`,
  `.AppImage`; a janela abre um `.minder` passado como argumento) e `nodo --mcp` testado de ponta a ponta.
- **CI/Release:** `.github/workflows/build.yml` (verificação + instaladores dos três sistemas nos Artifacts) e
  `release.yml` (rascunho → publicar); `scripts/check-version.sh` e `scripts/bump-version.sh`; guia em
  `docs/RELEASE.md`.
- **Site:** `site/` (index, privacidade, prints de `ui/scripts/site-shots.mjs`), na diagramação nova (D13). Publicado
  pelo workflow `.github/workflows/site.yml` (`wrangler pages deploy site --project-name=nodo`) a cada push no `main`
  que mexa em `site/`, sem depender da conexão do painel do Cloudflare com o GitHub (que se perdeu na troca de nome).
  Precisa dos segredos `CLOUDFLARE_API_TOKEN` (Cloudflare Pages: Edit) e `CLOUDFLARE_ACCOUNT_ID`; sem eles o job só
  avisa. Também roda à mão em Actions → Site → Run workflow.
- **Edição XMind e temas de mapa** (`lib/maptheme.ts`, `components/MapThemeSheet.svelte`, `Action::SetMapStyle`,
  `set_map_theme` no MCP, `set_map_style` no app); barra do macOS integrada (`setup` + `mac_lights_width` em
  `src/lib.rs`, `main.ts`, `map.css`).
- **Renomeado para Nodo** (0.2.0). Edição rápida de novo (`Canvas.svelte`:
  `scheduleRender`, 12–26 ms por Tab/Enter na prévia, um desenho só). Botões do macOS pela configuração da janela.
- **Atualização automática** (D19) e `.github/workflows/check-secrets.yml` (confere chave e token sem mostrar nada).
  O Release exige o segredo da chave.
- **Revisão antes da 0.2.0:** letras digitadas logo após Tab/Enter iam para o nó anterior ou se perdiam
  (`doc.type` guarda o que chega enquanto o nó nasce; `App.svelte` usa ele); "Procurar atualizações" só aparece onde
  o app se atualiza (`update_supported`); barra sem sobreposição em janela de 390 px (Novo e Abrir saem abaixo de 480).
- **Sites ligados:** seção "Do mesmo autor" no site do Nodo (leva ao Ideario) e no do Ideario (leva ao Nodo), com
  print do outro app e link no rodapé. `.github/workflows/delete-release.yml` apaga um Release antigo ou a tag que
  sobrou (recusa o último).
- **Repositórios apt e dnf** (D20): `scripts/linux-repos.sh` (cópia do Ideario), `.github/workflows/linux-repos.yml`
  (chamado pelo Release e à mão), `site/_redirects`, `site/linux/` (chave pública e `nodo.repo`), seção Linux no site e
  no README. Segredo `NODO_GPG_PRIVATE_KEY`, conferido no `check-secrets.yml`.
- **Atalhos** (D21): ⌘0 tamanho real (centrado na seleção), ⌘=/⌘- zoom, ⇧1 ver o mapa inteiro, ⇧2 enquadrar a
  seleção com o ramo visível (`Canvas.svelte`: `fit`, `fitSelection`, `actualSize`), ⌥↑/⌥↓ sobe/desce entre irmãos
  (`Action::Reorder` no núcleo, `reorder_node` no app, `move_sibling` no MCP; na raiz só troca com o mesmo lado),
  Home seleciona a raiz. O rótulo de zoom é botão para 100%. A lista de atalhos fica nas Configurações
  (seção Atalhos, em grupos), não na barra lateral (pedido do usuário).
- **Menus de contexto** (`lib/mapmenus.ts`, `components/MenuAt.svelte`/`MenuList.svelte` e `lib/menu.ts` copiados do
  Ideario): clique direito no nó e no fundo, tecla de menu e ⇧F10; cada item mostra o atalho. `ui.themeOpen` abre a
  folha Tema do mapa de qualquer lugar.
- **Estruturas e espaçamento** (D22): `nodo_core::STRUCTURES` (mapa, logico, organograma, arvore) e `SPACINGS`
  (compacto, normal, largo) em `theme.structure`/`theme.spacing` (atributos `nodo-structure`/`nodo-spacing`);
  `Action::SetStructure`/`SetSpacing`; `layout.rs` com `Mode` (PerNode, Right, Down, Tree) e folgas escaladas; cada nó
  recebe o layout do Minder mais parecido. App: `set_structure`, `set_spacing`, `relayout`; MCP: `set_layout`.
  Interface: folha "Mapa" (`MapThemeSheet.svelte`) com esquemas SVG (`lib/structures.ts`), espaçamento e "Arrumar
  posições"; submenu no menu do fundo; ligações verticais e em cotovelo em `render.ts`. A prévia espelha tudo.
- **Exportação** (D23): `nodo_core::export` (Markdown, OPML, HTML com tópicos que abrem e fecham e o marcador
  `<!-- nodo:figure -->`), `nodo_core::write_atomic`; app `export_text`/`write_export`; MCP `export_map` (texto, e grava
  se vier `path`). Interface: `lib/exporter.ts` (PNG em 1–3×, SVG pelo `lib/svgctx.ts`, um "canvas" que escreve SVG
  com o mesmo `draw` da tela; PDF de uma página pelo jsPDF carregado sob demanda, ~300 dpi, orientação automática pelo
  formato do mapa ou escolhida, A4/Carta; HTML = página do núcleo com o SVG dentro), folha `ExportSheet.svelte`, botão
  na barra, ⌘⇧E e item no menu do fundo. A folha oferece PNG, SVG, PDF, Markdown e OPML (o usuário dispensou o HTML,
  que fica só no núcleo e no MCP).
- **Importar Markdown e OPML** (D24): `nodo_core::import` (`from_markdown`: títulos e itens de lista em qualquer recuo
  viram tópicos, o resto vira nota, um `# Título` único vira a raiz; `from_opml`: `text`/`_note`, um `<outline>` único
  vira a raiz), `Editor::open_or_import` (por extensão; o mapa importado fica sem caminho e sujo, então salvar pergunta
  onde e o arquivo original nunca é sobrescrito). Usado pelo `open_map` do app e do MCP; o diálogo Abrir aceita
  `.md`, `.markdown`, `.txt` e `.opml`. Ida e volta: o Markdown e o OPML exportados voltam à mesma árvore.
- **Notas** (D25): nó com nota vira **o card do Ideario** (opção A do usuário: card na cor do ramo bem clara):
  `lib/cardtext.ts` (`cardHtml` = marcação do `NoteCard.svelte` do Ideario com os blocos `pv-*` e os limites do
  `preview_of`; hoje o `CardBody` do Atrio; `layoutCard` desenha o mesmo card no canvas/SVG para exportar), medido no
  DOM (`measureCard` em `measure.ts`, montando o `CardBody`) e posto por cima do canvas em `Canvas.svelte` (`.map-card`, sem mouse; o canvas desenha ramos e
  seleção). Duplo clique no título edita o título; no corpo, abre a nota. A nota se escreve num card flutuante ao
  lado do nó (`components/NoteCard.svelte`, translúcido, Markdown formatado com `lib/markdown.ts`, um clique edita,
  salva sozinho; `lib/notes.svelte.ts`). **Notas soltas** no fundo do mapa: `model::FreeNote`, `<nodo-notes>` no
  arquivo (o Minder ignora), `Action::AddFreeNote/SetFreeNote/PlaceFreeNote/RemoveFreeNote`, comandos
  `add_free_note`/`set_free_note`/`place_free_note`/`remove_free_note`, ferramentas MCP; "Nova nota aqui" no menu do
  fundo, "Adicionar nota" no do nó.
- **Atrio no Nodo** (D26): `main.ts` importa `@wylho/atrio/atrio.css` (o `ideario.css` saiu); cards pelo `CardBody`;
  ⌘C num nó copia como card e ⌘V cola um card (do Ideario ou de outro nó) como filho com a nota
  (`Action::AddTopic`, comando `add_topic`, `add_node` do MCP aceita `note`), e "Copiar como card"/"Colar card"
  nos menus. A cor da nota vira a cor do ramo e volta. Desde o Atrio 0.3.0, `MenuAt`, `MenuList`, `Picker` e
  `SEP`/`MenuEntry` vêm de lá (as cópias saíram de `components/` e `lib/menu.ts`).
- **Formato `.nodo`** (D28): `nodo-core/src/nodofile.rs` (testes de ida e volta, versão nova recusada, campos
  desconhecidos ignorados; proptest em `tests/layout_props.rs`), `Editor::open_or_import` (`.minder` vira importação;
  sem extensão conhecida, decide pelo conteúdo) e `Editor::save` (sempre `.nodo`). App: diálogos e
  `tauri.conf.json`; MCP: `open_map`/`save_map` e instruções. Site e README sem o Minder como argumento.
- **Card do Atrio no tópico** (D29): `model::CardFields` (etiquetas, emoji, lembrete, status, limpos por
  `atrio::text`; crate `atrio` por git na tag, reexportada em `nodo_core::atrio`), no `.nodo` dentro do tópico,
  `Action::SetCard` e `AddTopic` com cor e campos (colar card = um passo); MCP `set_card` e `card` no `get_node`; app
  `set_card`, `note_doc`/`set_note_doc`/`markdown_doc` (nota ↔ documento do TipTap pelo `atrio::markdown`, as regras
  do Ideario). Interface: `components/NodeNote.svelte` (o card aberto, barra do topo como a do Ideario),
  `components/MapCard.svelte`, `lib/notes.svelte.ts` (um card aberto por vez, `docMode`), `lib/notedoc.ts` (conversão
  só da prévia); copiar/colar leva emoji e etiquetas. `vite.config.ts` exclui o Atrio do pré-empacotamento.
  Seletor de status no card do mapa (Atrio 0.9.0, `CardPills` com `onstatus`; `doc.setStatus` com o `setStatus` do
  Atrio; `.map-card .status-toggle` recebe o clique). Painel lateral removido (pedido do usuário: ficou inútil).
  Falta: a exportação (PNG/SVG/PDF) ainda não desenha emoji e pílulas do card; anexos no `.nodo`.
- **Abas** (pedido do usuário: vários mapas ao mesmo tempo, abas no canto de cima do mapa, sem mexer na barra):
  `nodo-app/src/tabs.rs` (`Tabs`: um `Editor` por aba, id estável; abrir um arquivo ocupa o lugar de um mapa novo
  intocado, e um arquivo já aberto traz a aba dele; fechar traz a vizinha), comandos `switch_tab`/`close_tab`,
  `new_map` abre aba, snapshot com `tabs`/`active`, fechar o app olha todas as abas. Interface:
  `components/TabBar.svelte` (no desenho das abas do shadcn com o Atrio: trilho apagado e a aba da frente em relevo,
  espaço igual entre as abas, × na da frente e ao passar o mouse, ponto = não salvo; `data-mode` no `<html>` para o
  escuro; botão do meio fecha; + novo),
  `doc.switchTab`/`closeTab`/`cycleTab` (seleção de cada aba guardada), zoom e posição por aba (`doc.views`,
  `Canvas.svelte`), atalhos ⌘T/⌘N, ⌘W, Ctrl+Tab/Ctrl+⇧Tab. A prévia tem abas também. **Caixa de três saídas**
  (pedido do usuário): fechar aba ou app com algo por salvar oferece Salvar / Cancelar / Fechar sem salvar
  (`ui.choose`, `ConfirmDialog.svelte`); sair com várias abas não salvas oferece "Salvar todos".
- **Anexos e abas** (depois da 0.3.0): abas no canto do mapa (`src/tabs.rs`, `TabBar.svelte`, fechar com
  Salvar/Cancelar/Fechar sem salvar); anexos (D30): `crates/nodo-app/src/attachments.rs` (`import_file` com o corpo
  cru, `import_path`, `get_attachments`, `save_attachment`, `open_attachment`, protocolo `att://` com Range),
  `linux_media` libera câmera e microfone no WebKitGTK; card com foto, arquivo, câmera e gravador
  (`NodeNote.svelte`), capa no card do mapa (`coverFromMarkdown` do Atrio 0.10.0 em `MapCard.svelte`). ⌘/Ctrl+Enter
  abre o card do tópico. Atrio 0.11.0: anexo com o cursor numa lista entra depois dela.
- **Atrio 0.12.0** (unificação, fase 1: o Ideario passou a usar a mesma crate; plano em `atrio/docs/UNIFICACAO.md`).
- **Unificação, fase 2 (Atrio 0.13.0):** o núcleo usa as regras do Atrio, as mesmas do Ideario:
  - **anexos** (`nodo-core/src/attachments.rs` sobre `atrio::attach`): hash = SHA-256 inteiro; foto vira WebP com
    miniatura, paleta e proporção (`atrio::media`); a miniatura vai no `.nodo` em `miniaturas/<hash>.webp`
    (`package.rs`) e o `att://…?thumb` a serve (`nodo-app/src/attachments.rs`);
  - **status** (`CardFields::set_status(to, now)` com `atrio::card`): "feito" num lembrete que repete pula para a
    próxima vez, também pelo MCP;
  - **a prévia do card** (`nodo-core/src/preview.rs`: `card_view` = Markdown → documento → `atrio::projection`), no
    snapshot como `view` (blocos, capa com tamanho, contagem de fotos e anexos, `#tags`); `MapCard.svelte` usa a
    `view` (capa na proporção da foto e `+N`, #etiquetas do texto viram pílula, pílula de anexos); só a prévia do
    navegador (`lib/preview.ts`) usa a versão em TypeScript do Atrio;
  - **busca sem acento** (`atrio::text::fold`) na janela e no MCP.
- **Unificação, fase 3 (Atrio 0.15.1), em andamento:** o card do tópico é o `NoteCard` do Atrio, o mesmo do Ideario
  (`Canvas.svelte`; `lib/cardtext.ts` `summaryOf`; `MapCard.svelte` saiu), na cor de nota do Atrio mais próxima da
  cor do ramo (`nearestNoteColor`). Botão de recolher/expandir no começo do ramo (`render.ts` `drawKnob`/`hitKnob`,
  `hidden` no snapshot = tópicos escondidos), opção A do usuário (como o XMind): "−" ao passar o mouse ou com o tópico
  escolhido, recolhido mostra sempre a bolinha com o número; em tela de toque (`hover: none`) o "−" fica sempre à
  vista. A bolinha de nota da raiz saiu (a raiz abre o card com duplo clique, como os outros tópicos).
  **A nota solta é o card padrão** (pedido do usuário: o mapa é um canvas, a nota é a nota padrão): `FreeNote` com
  `title`, `color` (cor de nota do Atrio, `NOTE_COLORS`) e `card` (`CardFields`), no `.nodo` como num tópico;
  `Action::SetFreeCard`; app `free_note_doc`/`set_free_note_doc`/`set_free_card` e `view` (prévia) no snapshot; MCP
  `add_free_note` com `title` e `set_free_note` com cor e campos do card (`apply_card_args`, a mesma leitura do
  `set_card`). No mapa, o `NoteCard` do Atrio (`Canvas.svelte`: arrastar, duplo clique abre, menu Abrir/Apagar);
  aberta, o mesmo `NodeNote.svelte` do tópico (`target` = tópico ou nota solta); "Nova nota aqui" já abre o card.
  O `components/NoteCard.svelte` antigo (o card translúcido) saiu.
  A barra do card aberto é a `CardTopBar` do Atrio 0.16.0 (a mesma do Ideario; `NodeNote.svelte` só passa o rótulo
  e "Apagar tópico/nota" no `menuEnd`).
  Anexo aberto no card = o `AttachmentViewer` do Atrio 0.17.1 (`@wylho/atrio/viewer`, o mesmo do Ideario: foto com
  paleta, leitor de PDF, vídeo, áudio, Baixar); o comando `open_attachment` (abria no programa do sistema) saiu.
  `pdfjs-dist` entrou como dependência da interface.
  **Exportar desenha o próprio card** (`lib/domcard.ts`): o `NoteCard` do Atrio é montado fora da vista e o que o
  navegador desenhou vira operações (caixas com cantos, linhas de texto, fotos recortadas, ícones, as bolinhas do
  status pelo `::after`, marcadores `::before`), com as cores passadas a `rgb()`; `render.ts` (`replay`) repete no
  canvas (PNG/PDF) e no `SvgContext` (que ganhou `clip`, `drawImage` e `drawSvgMarkup`). Notas soltas e a bolinha de
  recolhido com o número também saem; a moldura usa a altura real do card da nota solta. O desenho à mão antigo
  (`layoutCard`, `measureRun`, `cardFill`) saiu. **Opções novas na folha** (pedido do usuário): Cards "Inteiros / Só
  o título" e Ramos recolhidos "Como estão / Expandir tudo" (só aparece se há ramo recolhido): o núcleo arruma o mapa
  para a imagem e devolve exatamente como estava (`Editor::stash_view`/`restore_view`, comando `export_layout`,
  `exportView` em `exporter.ts`); o documento, o desfazer e o "não salvo" não mudam.
  Fase 3 da unificação concluída no Nodo.
- **Próximo:** a exportação desenhar emoji e pílulas do card; anexos e gravações no tópico (precisam de um lugar
  para os arquivos ao lado do `.nodo`); o Ideario usar `CardPills`/`CardDialog` do Atrio no lugar das cópias dele;
  estruturas do XMind que faltam (chaves, linha do tempo, espinha de peixe, matriz, tabela em árvore); aba Estrutura
  na barra lateral com colar lista; abrir `.xmind`.

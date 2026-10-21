# Nodo — Especificação do projeto

Derivado do [Minder](https://github.com/phase1geo/Minder) 2.0.9 (GPL-3.0-or-later) por Trevor Williams.
Este documento é a fonte da verdade do projeto; `CLAUDE.md` resume as regras do dia a dia.

---

## 1. Visão

Um aplicativo de mapas mentais que abre rápido, guarda tudo em arquivo local e deixa um agente de IA trabalhar no
mesmo mapa que a pessoa — pelas mesmas regras, com o mesmo desfazer.

**Público:** quem faz mapas mentais no computador e quer algo nativo, leve e sem conta em serviço nenhum.

**Plataformas:** macOS, Linux e Windows.

### Princípios que decidem empates

1. **O arquivo é a verdade.** Nada de banco escondido, nada de estado só na memória. Fechar o app e abrir de novo mostra
   exatamente o que estava salvo.
2. **Compatibilidade com o Minder.** Um mapa feito no Minder abre aqui e volta sem perda. Isso limita escolhas de
   formato e é intencional: os mapas das pessoas valem mais que a nossa conveniência.
3. **Um núcleo, duas portas.** Interface e MCP não podem divergir. Toda regra de negócio vive em `nodo-core`; tanto o
   app quanto o agente passam por ela.
4. **Nada que não funcione aparece.** Sem botão decorativo, sem menu vazio.
5. **A tela não mente sobre o modelo.** Se o núcleo mede de um jeito e desenha de outro, o defeito é do código, não do
   usuário.

---

## 2. Stack decidida

| Camada | Escolha | Por quê |
|---|---|---|
| Janela | Tauri 2 | Leve, sem Chromium embutido, alvos de instalador prontos |
| Núcleo | Rust (edição 2021, MSRV 1.80) | Desempenho, e o mesmo código serve ao app e ao MCP |
| XML | `quick-xml` | O formato do Minder é XML com atributos irregulares; derive não cobre |
| Erros | `thiserror` no núcleo, `Result<T, String>` na borda | O destino final é sempre texto na interface |
| Interface | Svelte 5 (runes) + Vite + TypeScript | Mesma stack do Ideario; componentes sem runtime pesado |
| Testes do front | Vitest + jsdom | Rápido, sem navegador |
| Empacotamento | `cargo tauri build` | Gera `.app`/`.dmg`, `.deb`/`.rpm`/`.AppImage`, `.msi`/`.exe` |

---

## 3. Funcionalidades

### 3.1 Mapa
Uma raiz, filhos em árvore, ramos dos dois lados da raiz. Cada nó tem texto e, opcionalmente, uma nota em Markdown.
Nós podem ser coloridos por ramo, dobrados e arrastados.

### 3.2 Layout
Sete modos herdados do Minder: `Horizontal`, `Vertical`, `Up`, `Down`, `Left`, `Right`, `Manual`. O posicionamento
espelha o do Minder: o pai se centra na extensão dos filhos, irmãos são separados por uma margem fixa, o lado decide o
eixo de crescimento.

### 3.3 Seleção e edição (como no XMind)
Sempre há um nó selecionado (a raiz, ao abrir). Com um nó selecionado, qualquer letra começa a escrever nele,
substituindo o texto; Espaço, F2 ou duplo clique editam o texto atual. Tab cria um filho e Enter um irmão, já em
edição com "Nova ideia" selecionado; na edição, Enter confirma, Tab confirma e cria um filho, Esc cancela. Setas andam
para o vizinho geometricamente mais próximo; ⌘/ (Ctrl+/) recolhe. Uma dica no canto mostra o que o teclado faz agora.
Painel lateral para texto, nota, cor, lado do ramo e métricas.

### 3.4 Desfazer
Toda edição entra num histórico de instantâneos, com limite de 200 passos: texto, nota, cor, lado, recolher, mover e
rearranjar. Medidas vindas da tela não são edição (não entram no histórico nem marcam o mapa como alterado). Desfazer
até o estado salvo deixa o mapa sem nada por salvar. Arrastar só conta como edição depois de 2 px de movimento. O agente
que edita por MCP usa o mesmo `Editor`, com o mesmo histórico dentro da sessão dele.

### 3.5 Temas e linhas
Quatro paletas nomeadas (Nodo, Papel, Grafite, Floresta; a primeira é a paleta-base do Ideario com o nome do app) e um tema **Sistema** que imita GNOME, KDE, Windows ou macOS,
cada uma em claro e escuro. As cores de nó saem da própria paleta, então trocar de tema troca os ramos junto.
`fitAccent` ajusta um destaque do sistema até ele ter contraste suficiente contra a superfície.
As linhas entre nós podem ser **curvas** (Bézier) ou **em ângulo** (degrau com cantos arredondados), nas Configurações.

**Temas de mapa** (como os do XMind, `ui/src/lib/maptheme.ts`): Clássico, Contorno, Sublinhado, Vibrante, Pílulas,
Monocromático. Cada um define forma, recheio, contorno, texto e tamanho por nível (raiz, ramos principais, sub-ramos),
a paleta dos ramos e a espessura das linhas. Cada ramo principal recebe uma cor da paleta e os filhos herdam; a cor
escolhida no painel vale por cima. O tema é do mapa: fica no `.minder` como atributo `nodo-style` do `<theme>`
(ausente no Clássico, para o arquivo continuar igual ao do Minder), muda pelo histórico (`Action::SetMapStyle`) e o
núcleo só aceita os nomes de `MAP_STYLES`. Pelo MCP: `set_map_theme`.

### 3.6 Servidor MCP
`nodo --mcp` fala JSON-RPC sobre stdio e oferece 18 ferramentas (criar, abrir, salvar, ler, adicionar, remover,
editar texto e nota, cor, lado, tema do mapa, recolher, rearranjar, buscar, desfazer, refazer, estatísticas). Quando o agente salva o
arquivo que está aberto na janela, a janela relê ao voltar o foco (perguntando antes, se houver algo não salvo nela). Negocia a versão do protocolo e
envia `instructions` descrevendo o modelo de dados. O registro no Claude Desktop é feito pela própria interface, com as
garantias da §4.4.

### 3.7 Importação e exportação — **Fase 3, não implementado**
Markdown, OPML, FreeMind, Freeplane, Mermaid, SVG, PNG. O Minder tem 21 formatos; a ordem de ataque é por utilidade,
não por paridade.

---

## 4. Arquitetura

```
crates/
├── nodo-core/     modelo, layout, I/O do .minder, desfazer — sem interface
├── nodo-mcp/      protocolo MCP sobre stdio
└── nodo-app/      janela Tauri + interface Svelte
    └── ui/          HTML/CSS/JS
```

### 4.1 Um núcleo, duas portas

`nodo-core` não conhece janela, fonte ou canvas. O aplicativo e o agente usam o mesmo `nodo_core::Editor`, que é o
dono do mapa, do histórico, do arquivo e do estado salvo; a única diferença é por onde a chamada entra. Isso é o que
garante que uma edição por MCP seja indistinguível de uma feita à mão. Nenhuma das portas guarda uma cópia do mapa ao
lado do histórico: foi assim que recolher, mudar de lado e medidas se perdiam antes do `Editor`.

O layout reserva uma faixa para cada sub-árvore, põe cada ramo da raiz do lado dele e não posiciona descendentes de nó
recolhido. Propriedades com dados gerados (`nodo-core/tests/layout_props.rs`): nós visíveis nunca se sobrepõem, cada
ramo cresce para o seu lado, o XML vai e volta, desfazer tudo volta ao início.

### 4.2 Um binário, dois modos

```rust
fn main() {
    if std::env::args().skip(1).any(|a| a == "--mcp") {
        std::process::exit(nodo_mcp::serve());
    }
    nodo_app::run()
}
```

O instalador leva um arquivo só, e o caminho do agente usa provadamente o mesmo núcleo da janela.

### 4.3 Geometria e zoom

Medição e desenho acontecem em **unidades de mundo**. O zoom entra uma única vez:

```js
ctx.setTransform(dpr * scale, 0, 0, dpr * scale, dpr * x, dpr * y)
```

Misturar os dois espaços — medir em tamanho cheio e desenhar reduzido — faz o texto não caber na própria caixa e ser
truncado. Foi o defeito mais custoso já corrigido neste projeto; a regra existe para não voltar.

O texto é medido no frontend, com métricas reais, e enviado ao núcleo por `set_sizes`. O Rust apenas posiciona. Uma
crate de fontes no núcleo divergiria do que a tela desenha.

### 4.4 I/O de arquivo alheio

Ao registrar-se na configuração de um cliente MCP, o Nodo:
- **recusa** um arquivo que não seja JSON válido, deixando-o intacto (e diz isso a quem chamou);
- escreve num arquivo temporário e renomeia por cima, para a configuração nunca ser lida pela metade;
- preserva todas as outras chaves.

Ambas as regras têm teste (`nodo-mcp/src/setup.rs`).

---

## 5. Modelo de dados

O `.minder` é XML do Minder 2.0.9 (`<minder version="1.15.0">`). O que o Nodo modela:

| Elemento | Campos |
|---|---|
| `<minder>` | `version` |
| `<theme>` | `name`, `label`, `index`; `nodo-style` (tema do mapa, só do Nodo) |
| `<styles><style>` | 20 atributos por nível (margens, raios, fontes, larguras, traços, setas) |
| `<drawarea>` | `x`, `y`, `scale` |
| `<nodes><node>` | `id`, `posx`, `posy`, `width`, `height`, `side`, `fold`, `treesize`, `layout`, `group`, `color`, `colorroot` |
| `<nodename><text>` | `data` |
| `<nodenote>` | texto no corpo do elemento |

Atributos desconhecidos são ignorados na leitura, para que arquivos de versões mais novas do Minder ainda abram.

---

## 6. Interface

### 6.1 Estrutura

Barra superior (64 px, igual à do Ideario), palco com o canvas, painel lateral de 320 px. Abaixo de 900 px o painel
vira gaveta inferior.

### 6.2 Tokens

Os tokens são os do Ideario, copiados de `src/lib/palettes.ts`. Aplicados como variáveis inline no `<html>`, cerca de
25 no total. O design system é `ui/src/ideario.css`, **cópia literal** do `app.css` do Ideario (não editar; copiar de
novo quando o Ideario mudar); `ui/src/map.css` tem só o que é de mapa (canvas, zoom, inspetor).

### 6.2.1 Estrutura (padrão dos nossos apps)

A casca segue a do Ideario: barra superior (`TopBar`) com o que é do documento à esquerda e, ancorado à direita, o que
está sempre lá (busca que abre a partir do ícone, painel, Configurações); as ações do nó selecionado ficam à esquerda
disso e só aparecem com seleção. Configurações numa folha (`SettingsSheet`, bits-ui) com Aparência, Mapa e Claude.
Avisos no `toast` e confirmação no `ConfirmDialog` do Ideario; ícones do `@lucide/svelte`. O menu de contexto do
navegador fica bloqueado fora de campos de texto.

### 6.3 Preview

`ui/src/lib/preview.ts` implementa os comandos do núcleo sobre um mapa de exemplo em memória. Com `?preview`, a
interface real roda no navegador com HMR — é o caminho obrigatório para validar qualquer mudança visual antes de
compilar o Tauri, que leva minutos e não deixa inspecionar nada.

---

## 7. Plataformas

| Plataforma | Instalador | Estado |
|---|---|---|
| macOS | `.dmg` (universal) | Funciona; sem assinatura da Apple (P4) |
| Linux | `.deb`, `.rpm`, `.AppImage` | `.deb` e `.AppImage` compilados e abertos no Linux |
| Windows | `-setup.exe`, `.msi` | Pelo workflow; ainda não executado |

Os instaladores associam a extensão `.minder` ao Nodo: dois cliques abrem o mapa (argumento no Windows e no Linux,
evento de abertura no macOS). Guia de lançamento: `docs/RELEASE.md`.

---

## 8. Site

`site/` é publicado em **nodo-1c7.pages.dev** pelo Cloudflare Pages (pasta estática, sem build), com a mesma estrutura e
o mesmo `style.css` do site do Ideario. O `README.md` tem o mesmo conteúdo (skill `sync-site`); os prints saem da
prévia por `crates/nodo-app/ui/scripts/site-shots.mjs`, com o mapa de exemplo inventado.

## 8.1 Protótipo de design

`prototype/` guarda os protótipos em HTML que antecederam a migração para Svelte, incluindo `theme.js`, o porte das
paletas. Servem de referência histórica; a implementação viva é `crates/nodo-app/ui/src/`.

---

## 9. Fases de implementação

Cada fase termina com algo verificável.

| Fase | Entrega | Pronto quando | Estado |
|---|---|---|---|
| **1. Núcleo** | Modelo, layout, I/O do `.minder`, desfazer, servidor MCP | Um mapa real do Minder abre, é editado e volta sem perda | ✅ |
| **2. Janela** | Tauri + interface Svelte, temas, edição, MCP no mesmo binário | O app abre, desenha, edita e salva; o agente edita o mesmo mapa | ✅ |
| **3. Intercâmbio** | Importação e exportação (Markdown, OPML, FreeMind, Mermaid, SVG, PNG) | Um mapa vai para Markdown e volta; um SVG sai legível | ⬜ |
| **4. Escala** | Conexões desenháveis, dobra, busca avançada, mapas grandes | Um mapa de 5 mil nós rola a 60 fps | ⬜ |
| **5. Polimento** | Animações, modo foco, abas, menu nativo, i18n | Uso diário sem atrito | ⬜ |

---

## 10. Decisões pendentes (perguntar antes da fase correspondente)

- **P1 — Conexões entre nós.** O modelo já lê e escreve conexões, mas a interface não as desenha nem permite criá-las.
  Desenhar agora (Fase 4) ou tratar como parte de "escala"?
- **P2 — Múltiplos mapas abertos.** Abas como no Minder, ou uma janela por mapa?
- **P3 — Formato próprio.** Manter só `.minder` ou aceitar um formato nativo também? (D5 diz manter `.minder`; revisitar
  se a Fase 3 mostrar atrito.)
- **P4 — Assinatura de código.** Sem certificado, o macOS avisa "desenvolvedor não identificado". Assinar exige conta
  Apple paga; decidir antes de publicar instalador para terceiros.
- **P5 — Atualização automática.** Como no Ideario (`tauri-plugin-updater`). Precisa de uma chave de assinatura guardada
  pelo usuário; quem instalar uma versão sem o atualizador terá de baixar a seguinte à mão.

---

## 11. Fora do escopo (por enquanto)

Colaboração em tempo real, sincronização por nuvem, conta de usuário, versão web pública, aplicativo móvel.

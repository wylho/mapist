# Mapist — Especificação do projeto

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
3. **Um núcleo, duas portas.** Interface e MCP não podem divergir. Toda regra de negócio vive em `mapist-core`; tanto o
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

### 3.3 Seleção e edição
Seleção por clique; navegação por setas (vizinho geometricamente mais próximo na direção); edição no lugar por
duplo-clique; painel lateral para texto, nota, cor e métricas.

### 3.4 Desfazer
Toda edição entra num histórico de instantâneos, com limite de 200 passos. Arrastar só conta como edição depois de 2 px
de movimento, para um clique não poluir o histórico. O agente que edita por MCP usa o mesmo histórico.

### 3.5 Temas
Quatro paletas nomeadas (Ideario, Papel, Grafite, Floresta) e um tema **Sistema** que imita GNOME, KDE, Windows ou macOS,
cada uma em claro e escuro. As cores de nó saem da própria paleta, então trocar de tema troca os ramos junto.
`fitAccent` ajusta um destaque do sistema até ele ter contraste suficiente contra a superfície.

### 3.6 Servidor MCP
`mapist --mcp` fala JSON-RPC sobre stdio e oferece 16 ferramentas (criar, abrir, salvar, ler, adicionar, remover,
editar texto e nota, cor, lado, rearranjar, buscar, desfazer, refazer, estatísticas). Negocia a versão do protocolo e
envia `instructions` descrevendo o modelo de dados. O registro no Claude Desktop é feito pela própria interface, com as
garantias da §4.4.

### 3.7 Importação e exportação — **Fase 3, não implementado**
Markdown, OPML, FreeMind, Freeplane, Mermaid, SVG, PNG. O Minder tem 21 formatos; a ordem de ataque é por utilidade,
não por paridade.

---

## 4. Arquitetura

```
crates/
├── mapist-core/     modelo, layout, I/O do .minder, desfazer — sem interface
├── mapist-mcp/      protocolo MCP sobre stdio
└── mapist-app/      janela Tauri + interface Svelte
    └── ui/          HTML/CSS/JS
```

### 4.1 Um núcleo, duas portas

`mapist-core` não conhece janela, fonte ou canvas. O aplicativo e o agente chamam as mesmas funções; a única diferença é
por onde a chamada entra. Isso é o que garante que uma edição por MCP seja indistinguível de uma feita à mão.

### 4.2 Um binário, dois modos

```rust
fn main() {
    if std::env::args().skip(1).any(|a| a == "--mcp") {
        std::process::exit(mapist_mcp::serve());
    }
    mapist_app::run()
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

Ao registrar-se na configuração de um cliente MCP, o Mapist:
- **recusa** um arquivo que não seja JSON válido, deixando-o intacto (e diz isso a quem chamou);
- escreve num arquivo temporário e renomeia por cima, para a configuração nunca ser lida pela metade;
- preserva todas as outras chaves.

Ambas as regras têm teste (`mapist-mcp/src/setup.rs`).

---

## 5. Modelo de dados

O `.minder` é XML do Minder 2.0.9 (`<minder version="1.15.0">`). O que o Mapist modela:

| Elemento | Campos |
|---|---|
| `<minder>` | `version` |
| `<theme>` | `name`, `label`, `index` |
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
25 no total. `app.css` mantém só o design system e as regras próprias do mapa.

### 6.3 Preview

`ui/src/lib/preview.ts` implementa os comandos do núcleo sobre um mapa de exemplo em memória. Com `?preview`, a
interface real roda no navegador com HMR — é o caminho obrigatório para validar qualquer mudança visual antes de
compilar o Tauri, que leva minutos e não deixa inspecionar nada.

---

## 7. Plataformas

| Plataforma | Instalador | Estado |
|---|---|---|
| macOS | `.app`, `.dmg` | Funciona, verificado |
| Linux | `.deb`, `.rpm`, `.AppImage` | Workflow escrito, não executado |
| Windows | `.msi`, `.exe` | Workflow escrito, não executado |

---

## 8. Protótipo de design

`prototype/` guarda os protótipos em HTML que antecederam a migração para Svelte, incluindo `theme.js`, o porte das
paletas. Servem de referência histórica; a implementação viva é `crates/mapist-app/ui/src/`.

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

---

## 11. Fora do escopo (por enquanto)

Colaboração em tempo real, sincronização por nuvem, conta de usuário, versão web pública, aplicativo móvel.

<!-- Mesmo conteúdo da página em português do site (site/pt/index.html) e do README.md em inglês. Ao mudar um, mude os outros: skill /sync-site. -->

<p align="center"><a href="README.md">English</a> · <b>Português</b></p>

<p align="center">
  <img src="site/favicon.svg" width="88" height="88" alt="Ícone do Nodo">
</p>

<h1 align="center">Nodo</h1>

<p align="center"><b>Pense em mapas.</b><br>
Um app de mapas mentais grátis e de código aberto que abre num instante, funciona sem internet e guarda cada mapa num arquivo seu,
legível sem o app. E o Claude pode trabalhar no mesmo mapa que você, pelas mesmas regras.</p>

<p align="center"><a href="https://github.com/wylho/nodo/releases/latest"><b>Versão 0.6.1: arquivos .nodo abrem pelo gerenciador de arquivos e o recolher fica claro em todas as estruturas →</b></a></p>

<p align="center">
  <b>Windows · macOS · Linux</b><br>
  <a href="#baixar"><b>Baixar</b></a> ·
  <a href="https://nodo-1c7.pages.dev/pt/">nodo-1c7.pages.dev</a> ·
  <a href="https://nodo-1c7.pages.dev/privacidade/">Privacidade</a> ·
  <a href="README.md">English</a> ·
  <a href="https://github.com/sponsors/wylho">♥ Apoiar</a>
</p>

<picture>
  <source srcset="site/img/pt/mapa-escuro.webp" media="(prefers-color-scheme: dark)">
  <img src="site/img/pt/mapa.webp" alt="Um mapa mental no Nodo: a raiz no centro, ramos coloridos dos dois lados, tópicos em cards com checklist, etiquetas e status, e uma nota solta">
</picture>

## Feito para pensar rápido

Tudo pelo teclado, nada de esperar. O mapa se arruma sozinho enquanto você escreve.

| | |
|---|---|
| **Selecione e digite** | O texto entra direto no nó. Tab cria um filho, Enter um irmão, já prontos para escrever; as setas andam pelo mapa. |
| **Dos dois lados** | Ramos à esquerda e à direita da raiz. Escolha o lado de cada um no clique direito. |
| **Notas em cada nó** | Texto curto no mapa, detalhes na nota em Markdown, que fica guardada junto. |
| **Desfazer de verdade** | Duzentos passos para trás e para a frente, sem perder o que você recolheu ou mudou de lado. |
| **Busca no mapa** | Encontra pelo texto ou pela nota, e marca cada nó que bate. |
| **Sem internet** | Nenhuma conta, nenhum servidor. Local-first: o mapa é um arquivo no seu computador. |

## O Claude no mesmo mapa

O Nodo traz um servidor MCP embutido, para agentes de IA como o Claude editarem seus mapas. Ligue nas Configurações e peça ao Claude para ler, criar e reorganizar seus
mapas. Ele usa o mesmo núcleo do app: cada edição dele segue as mesmas regras de uma feita à mão, e ele também desfaz
o que fez.

- Claude Desktop: um botão nas Configurações
- Claude Code: um comando para colar no terminal
- Quando o Claude salva, a janela recarrega o mapa ao voltar para ela

<img src="site/img/pt/claude.webp" alt="Configurações do Nodo com a aparência, o traçado das linhas e a seção Claude, com o botão Ligar e o comando do Claude Code">

## O arquivo é seu

Cada mapa é um arquivo `.nodo`: um zip com o mapa em JSON legível e os anexos, que abre sem o app. Salvar nunca deixa
um arquivo pela metade. Mapas `.minder`, Markdown e OPML abrem como mapa novo, e o original fica como está.

<img src="site/img/pt/busca.webp" alt="Mapa com a busca aberta na barra, marcando os nós que contêm a palavra Design">

## Temas de mapa

Seis temas, no espírito do XMind: cartões coloridos, contorno, texto sobre a linha, cores fortes, pílulas ou cinzas.
Cada ramo principal ganha uma cor e os de baixo herdam. O tema fica salvo no próprio mapa.

- Ramos em curva ou em ângulo, com cantos arredondados
- Uma cor escolhida à mão continua valendo por cima do tema

<img src="site/img/pt/temas.webp" alt="A folha Tema do mapa, com seis mapinhas de exemplo: Clássico, Contorno, Sublinhado, Vibrante, Pílulas e Monocromático">

## Com a cara do seu sistema

Temas claro e escuro, e um tema Sistema que imita as cores do Windows, do macOS, do GNOME e do KDE. Funciona também numa
janela estreita.

<img src="site/img/pt/janela-estreita.webp" width="300" alt="O Nodo numa janela estreita, no tema escuro, com zoom num card de tópico e o checklist dele">

## Estruturas e formatos

Troque a estrutura do mapa inteiro com um clique, e leve suas ideias para dentro e para fora em formatos abertos.

| | |
|---|---|
| **Sete estruturas** | Mapa mental, diagrama lógico, organograma, árvore, matriz, tabela em árvore e linha do tempo, com espaçamento compacto, normal ou largo. A estrutura fica salva no mapa e se desfaz. |
| **Importa e exporta Markdown e OPML** | Abra arquivos Markdown, OPML e do Minder como mapa novo. Exporte em PNG, SVG, PDF para imprimir, Markdown ou OPML. |
| **Cada tópico é um card** | Nota em Markdown, etiquetas, emoji, lembrete, status em três cores, fotos e anexos. Copie um card para o Ideario, ou cole um que veio de lá. |
| **Vários mapas em abas** | Deixe vários mapas abertos e troque com Ctrl+Tab. Fechar um com algo por salvar pergunta antes. |

## Baixar

Grátis e de código aberto. Os instaladores para Windows, macOS e Linux ficam na página de
[versões do GitHub](https://github.com/wylho/nodo/releases/latest). Depois de instalado, o Nodo avisa quando sai versão
nova e se atualiza sozinho.

| | |
|---|---|
| **Windows** | O `-setup.exe` (ou o `.msi`). |
| **macOS** | O `.dmg`, para Apple Silicon e Intel. Na primeira vez, clique com o botão direito no app e escolha **Abrir**: ele ainda não é assinado pela Apple. |
| **Linux** | Pelo repositório apt ou dnf, que atualiza junto com o sistema (abaixo). Ou o `.AppImage`, que roda em qualquer distribuição e se atualiza sozinho. |

### Linux, pelo repositório

Ubuntu, Debian e derivados:

```sh
curl -fsSL https://nodo-1c7.pages.dev/linux/nodo.gpg | sudo tee /usr/share/keyrings/nodo.gpg > /dev/null
echo "deb [signed-by=/usr/share/keyrings/nodo.gpg] https://nodo-1c7.pages.dev/apt stable main" | sudo tee /etc/apt/sources.list.d/nodo.list
sudo apt update && sudo apt install nodo
```

Fedora:

```sh
sudo curl -fsSL -o /etc/yum.repos.d/nodo.repo https://nodo-1c7.pages.dev/linux/nodo.repo
sudo dnf install nodo
```

**Privacidade:** sem anúncios, sem rastreamento, sem coleta de dados. Seus mapas ficam no seu computador. Leia a
[Política de Privacidade](https://nodo-1c7.pages.dev/privacidade/).

---

## Para desenvolver

Especificação: [`docs/SPEC.md`](docs/SPEC.md). Regras do projeto: [`CLAUDE.md`](CLAUDE.md). Lançar uma versão:
[`docs/RELEASE.md`](docs/RELEASE.md).

```
crates/
├── nodo-core/   modelo, layout, .nodo (e leitura de .minder), Editor (histórico + arquivo), sem interface
├── nodo-mcp/    servidor MCP sobre stdio (24 ferramentas)
└── nodo-app/    janela Tauri; a interface Svelte 5 fica em ui/
site/              nodo-1c7.pages.dev (Cloudflare Pages, sem build)
```

O aplicativo e o MCP passam pelo mesmo `nodo_core::Editor`: um agente produz exatamente o mesmo resultado que uma
pessoa na janela. Um binário, dois modos: `nodo` abre a janela, `nodo --mcp` fala o protocolo.

Requer Rust 1.80+, Node 22 e pnpm. No Linux, as bibliotecas do WebKitGTK
(`libwebkit2gtk-4.1-dev libayatana-appindicator3-dev librsvg2-dev patchelf`).

```bash
pnpm --dir crates/nodo-app/ui install

# Interface no navegador, sobre um backend simulado, com recarga instantânea: o caminho para qualquer mudança visual
pnpm --dir crates/nodo-app/ui dev          # abrir http://localhost:1420/?preview

# Verificação completa
cargo fmt --all --check && cargo clippy --all-targets -- -D warnings && cargo test --workspace
pnpm --dir crates/nodo-app/ui run check && pnpm --dir crates/nodo-app/ui run test

# O app de verdade (cargo build sozinho não embute a interface e abre uma janela vazia)
crates/nodo-app/ui/node_modules/.bin/tauri build
```

### MCP fora do app

```json
{ "mcpServers": { "nodo": { "command": "/caminho/para/nodo", "args": ["--mcp"] } } }
```

Ferramentas: `new_map`, `open_map`, `save_map`, `get_map`, `get_node`, `add_node`, `remove_node`, `set_text`,
`set_note`, `set_color`, `set_side`, `set_map_theme`, `toggle_fold`, `relayout`, `search_nodes`, `undo`, `redo`,
`map_stats`.

## Licença

GPL-3.0-or-later, herdada do Minder. Veja [LICENSE](LICENSE). O Minder é de Trevor Williams e colaboradores; o Nodo
é obra derivada e leva a mesma licença.

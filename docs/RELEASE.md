# Lançar uma versão

1. Suba a versão nos três lugares de uma vez: `scripts/bump-version.sh 0.2.0` (Cargo.toml do workspace,
   `crates/nodo-app/tauri.conf.json` e `crates/nodo-app/ui/package.json`). A do `tauri.conf.json` vira a tag
   `vX.Y.Z`; o CI recusa versões diferentes (`scripts/check-version.sh`).
2. Commit e push no branch padrão.
3. No GitHub: **Actions → Release → Run workflow** (no branch padrão).
4. O workflow verifica tudo de novo (clippy, testes do núcleo e da interface), compila os três sistemas num Release
   **rascunho** e, quando os três terminam, o job "Publicar" o publica como último Release. Uma falha em qualquer
   sistema deixa só o rascunho, que dá para apagar e refazer.

## O que sai em cada sistema

| Sistema | Arquivos | Observação |
|---|---|---|
| macOS (Apple Silicon e Intel, universal) | `.dmg` | Sem assinatura da Apple ainda (SPEC P4): na primeira vez, botão direito → Abrir |
| Windows | `-setup.exe` (NSIS) e `.msi` | O SmartScreen avisa enquanto o instalador não tiver assinatura |
| Linux | `.AppImage`, `.deb`, `.rpm` | Ícone sem placa, como os do GNOME |

Os mesmos instaladores saem a cada push nos "Artifacts" do workflow **Build**, para testar antes de lançar.

## Antes de lançar, à mão

- Abrir o instalador do Build no sistema principal (macOS) e conferir: abre, cria, salva, reabre o arquivo salvo.
- Configurações → Claude → Ligar, reiniciar o Claude Desktop e pedir para ele ler o mapa aberto.
- Um `.minder` feito no Minder abre e volta sem perda (`cargo run -p nodo-core --example parse_real arquivo.minder`).

## Atualização automática

Desde a 0.2.0, como no Ideario (`tauri-plugin-updater`, `src/updates.rs`): ao abrir, o app lê o `latest.json` do
último Release e, se houver versão nova, mostra um aviso no canto com **Atualizar**. Vale no macOS, no Windows e no
AppImage; o `.deb` e o `.rpm` não se atualizam sozinhos (repositório apt/dnf ainda não existe).

- O Release assina cada instalador com a chave dos segredos `TAURI_SIGNING_PRIVATE_KEY` e
  `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`; o app só instala o que a chave pública do `tauri.conf.json` confirma.
  Sem o segredo o workflow Release para logo no começo.
- **A chave privada não pode se perder**: sem ela, quem já instalou nunca mais recebe versão nova. A cópia fica no
  Ideario do usuário (categoria de chaves, com PIN), na nota "Nodo — chave das atualizações".
- Depois de criar ou trocar um segredo: Actions → **Conferir segredos** → Run workflow.

## Ainda não ligado
- **Assinatura de código** no macOS e no Windows (SPEC P4).
- **Repositórios apt/dnf** (o Ideario tem; aqui só se houver pedido).

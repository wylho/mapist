# Lançar uma versão

1. Suba a versão nos três lugares de uma vez: `scripts/bump-version.sh 0.2.0` (Cargo.toml do workspace,
   `crates/mapist-app/tauri.conf.json` e `crates/mapist-app/ui/package.json`). A do `tauri.conf.json` vira a tag
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
- Um `.minder` feito no Minder abre e volta sem perda (`cargo run -p mapist-core --example parse_real arquivo.minder`).

## Ainda não ligado

- **Atualização automática** (como no Ideario: `tauri-plugin-updater` + `latest.json` do último Release). Precisa
  de uma chave de assinatura gerada uma vez (`tauri signer generate`) e guardada como segredo
  (`TAURI_SIGNING_PRIVATE_KEY`, `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`) **e** fora do GitHub, num gerenciador de
  senhas: sem ela, quem já instalou nunca mais recebe versão nova. Quem instalar uma versão sem o atualizador vai
  precisar baixar a seguinte à mão.
- **Assinatura de código** no macOS e no Windows (SPEC P4).
- **Repositórios apt/dnf** (o Ideario tem; aqui só se houver pedido).

---
name: sync-site
description: Mantém o site do Nodo (site/, publicado em nodoapp.pages.dev pelo Cloudflare Pages) e a página do GitHub (README.md) com o mesmo conteúdo e prints atualizados. Use ao mudar o texto ou os prints de um dos dois, ao lançar uma versão nova, ao mudar um recurso que aparece na página inicial, ou quando o usuário pedir para atualizar o site ou o README.
---

# Site e README sincronizados

A página inicial do site (`site/index.html`) e o `README.md` contam a mesma coisa, com os mesmos prints
(`site/img/*.webp`). O site é a referência: mude primeiro o site, depois leve o mesmo conteúdo ao README.
A página `site/privacidade/` só existe no site (o README só aponta para ela). A estrutura e o `style.css` são os do
site do Ideario: quando o Ideario mudar o estilo, copiar o `style.css` de novo.

## Regra que não pode falhar: só dados fictícios

O repositório e o site são **públicos**. Os prints vêm da prévia do app, com o mapa de exemplo de
`crates/nodo-app/ui/src/lib/preview.ts` (`seedDemo`), que é inventado. Nunca:

- tirar print de um mapa do usuário, de um arquivo `.minder` dele ou do app instalado dele;
- pôr nos dados de exemplo nomes de clientes, empresas, pessoas ou qualquer coisa da vida do usuário;
- publicar um print sem antes abrir cada imagem e ler tudo o que aparece nela (título, nós, notas, painel).

Se aparecer algo que pareça real, pare e pergunte ao usuário antes de publicar.

## Quando o conteúdo muda

1. Edite `site/index.html` (português primeiro; a seção `#english` resume em inglês).
2. Leve a mesma mudança ao `README.md`, na mesma ordem de seções. Diferenças permitidas:
   - no README as imagens usam caminhos relativos (`site/img/...`) e não há `loading`/`width` de layout;
   - o README termina com "Para desenvolver" (só no README);
   - o comentário no topo do README fica.
3. Confira que as afirmações são verdadeiras no código (recurso existe, nome do botão igual ao da UI). Nada de
   prometer o que não existe; nada de botão de download direto sem um Release público para apontar.

## Quando os prints precisam mudar

```sh
pnpm --dir crates/nodo-app/ui build && pnpm --dir crates/nodo-app/ui site-shots
```

O script (`crates/nodo-app/ui/scripts/site-shots.mjs`) sobe a prévia e salva os prints em WebP em `site/img`
(precisa do `convert` do ImageMagick; o Chromium pode vir de `CHROMIUM=/caminho`). Para um print novo, acrescente uma
chamada `shot(...)` no script (não tire à mão). A prévia aceita `?select=Texto do nó` para abrir com um nó
selecionado. Depois **abra cada imagem** (reduzida, se preciso) e leia tudo.

## Publicação

O Cloudflare Pages publica a pasta `site/` do branch padrão a cada push (sem comando de build). Push com mudança em
`site/` no branch padrão = site no ar.

#!/bin/sh
# Atualiza os bancos do sistema para o .nodo aparecer com o Nodo em "Abrir com" (e some ao desinstalar).
update-mime-database /usr/share/mime >/dev/null 2>&1 || true
update-desktop-database -q /usr/share/applications >/dev/null 2>&1 || true
exit 0

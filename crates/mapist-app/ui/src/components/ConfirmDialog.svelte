<script lang="ts">
  import { AlertDialog } from 'bits-ui'
  import { ui } from '../lib/ui.svelte'

  // Confirmação só para o que não tem volta (o resto usa "Desfazer"). Mesmo componente do Ideario,
  // ligado ao pedido em `ui.confirmRequest`: fechar sem confirmar conta como cancelar.
  const req = $derived(ui.confirmRequest)

  function close(confirmed: boolean) {
    const r = ui.confirmRequest
    ui.confirmRequest = null
    if (confirmed) r?.onconfirm()
    else r?.oncancel?.()
  }
</script>

<AlertDialog.Root open={!!req} onOpenChange={(o) => { if (!o && ui.confirmRequest) close(false) }}>
  <AlertDialog.Portal>
    <AlertDialog.Overlay class="overlay" />
    <AlertDialog.Content class="confirm">
      <AlertDialog.Title class="confirm-title">{req?.title}</AlertDialog.Title>
      <AlertDialog.Description class="confirm-text">{req?.text}</AlertDialog.Description>
      <div class="confirm-actions">
        <AlertDialog.Cancel class="btn ghost">Cancelar</AlertDialog.Cancel>
        <AlertDialog.Action class="btn danger" onclick={() => close(true)}>{req?.confirm}</AlertDialog.Action>
      </div>
    </AlertDialog.Content>
  </AlertDialog.Portal>
</AlertDialog.Root>

<script lang="ts">
  import { AlertDialog } from 'bits-ui'
  import { t } from '@wylho/atrio'
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

  function save() {
    const r = ui.confirmRequest
    ui.confirmRequest = null
    r?.onsave?.()
  }
</script>

<AlertDialog.Root open={!!req} onOpenChange={(o) => { if (!o && ui.confirmRequest) close(false) }}>
  <AlertDialog.Portal>
    <AlertDialog.Overlay class="overlay" />
    <AlertDialog.Content class="confirm">
      <AlertDialog.Title class="confirm-title">{req?.title}</AlertDialog.Title>
      <AlertDialog.Description class="confirm-text">{req?.text}</AlertDialog.Description>
      {#if req?.save}
        <!-- Três saídas, como no sistema: descartar à esquerda, longe do Salvar, que é a principal. -->
        <div class="confirm-actions three">
          <button class="btn ghost discard" onclick={() => close(true)}>{req.confirm}</button>
          <AlertDialog.Cancel class="btn ghost">{t('Cancelar')}</AlertDialog.Cancel>
          <AlertDialog.Action class="btn primary" onclick={save}>{req.save}</AlertDialog.Action>
        </div>
      {:else}
        <div class="confirm-actions">
          <AlertDialog.Cancel class="btn ghost">{t('Cancelar')}</AlertDialog.Cancel>
          <AlertDialog.Action class="btn danger" onclick={() => close(true)}>{req?.confirm}</AlertDialog.Action>
        </div>
      {/if}
    </AlertDialog.Content>
  </AlertDialog.Portal>
</AlertDialog.Root>

<style>
  .three { flex-wrap: wrap; }
  .three :global(.btn.primary) { flex: none; }
  .three .discard { margin-right: auto; color: var(--late); }
</style>

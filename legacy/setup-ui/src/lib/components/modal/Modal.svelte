<script lang="ts">
  import Trigger from './ModalButton.svelte';
  import Dialog from './ModalDialog.svelte';

  const C = {
    Trigger,
    Dialog,
  };
</script>

<slot {C} />

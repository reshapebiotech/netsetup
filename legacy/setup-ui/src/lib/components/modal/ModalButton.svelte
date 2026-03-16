<script lang="ts">
  import Button from '../button/Button.svelte';

  export let id: string;

  const getElement: (id: string) => HTMLDialogElement | null = (id) =>
    document.getElementById(id) as HTMLDialogElement | null;
</script>

<Button on:click={() => getElement(id)?.showModal()}><slot /></Button>

<script lang="ts">
  export let id: string;
  export let open: boolean = false;
  export let modalBoxSx: string = '';
</script>

<dialog
  {id}
  class={`modal modal-bottom sm:modal-middle modal-open bg-background-500`}
  class:modal-open={open}
>
  <div class={`modal-box p-0 ${modalBoxSx} max-w-full`}>
    <slot />
  </div>
</dialog>

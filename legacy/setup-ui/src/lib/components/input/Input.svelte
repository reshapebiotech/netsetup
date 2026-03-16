<script lang="ts">
  export let placeholder: string = '';
  export let value: string = '';
  export let disabled: boolean = false;
  export let isPassword: boolean = false;
  export let name: string = '';
  export let validate: ((input: HTMLInputElement) => boolean) | undefined = undefined;

  const classes =
    'input input-ghost w-full bg-background-500 border-gray-51 hover:border-periwinkle-2 rounded-lg transition-colors focus:outline-none focus:ring-0 focus:border-periwinkle-5';
</script>

{#if isPassword}
  <input
    type="password"
    {placeholder}
    {disabled}
    {name}
    class={classes}
    bind:value
    on:input={(input) => validate && validate(input)}
  />
{:else}
  <input type="text" {placeholder} {disabled} {name} class={classes} bind:value />
{/if}

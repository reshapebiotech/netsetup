<script lang="ts">
  export let name: string = '';
  export let id: string = '';
  export let value: string = '';
</script>

<select
  class="select bg-background-500 border-gray-51 hover:border-periwinkle-2 w-full h-8 rounded-lg transition-colors focus:outline-none focus:ring-0 focus:border-periwinkle-5"
  {name}
  {id}
  bind:value
>
  <slot />
</select>

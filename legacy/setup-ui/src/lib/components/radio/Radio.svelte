<script lang="ts">
  export let group: string;
  export let value: string;
  export let id: string | undefined;
</script>

<input {id} type="radio" class="radio radio-primary" {value} bind:group />

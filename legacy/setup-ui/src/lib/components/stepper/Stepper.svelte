<script lang="ts">
  export let numberOfSteps: number;
  export let currentStep: number;
</script>

<div class="flex flex-row gap-3 mb-4">
  {#each Array.from({ length: numberOfSteps }, (_, i) => i + 1) as step}
    <div
      class={`flex-1 h-1.5 rounded bg-midnight-10 `}
      class:bg-primary-500={currentStep >= step}
    />
  {/each}
</div>

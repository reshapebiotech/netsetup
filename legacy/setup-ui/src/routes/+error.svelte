<script lang="ts">
  import Page from '$lib/components/page/Page.svelte';
</script>

<Page let:C>
  <C.Section>
    <C.Section>
      <C.Title>Something went wrong</C.Title>
      <C.Description
        >Please restart your device by removing and then reinserting the power plug.</C.Description
      >
    </C.Section>
  </C.Section>
  <div class="flex flex-col gap-10"></div>
</Page>

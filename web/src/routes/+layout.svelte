<script lang="ts">
    import '../app.css';
    let { children } = $props();
</script>

<div class="min-h-screen bg-zinc-950 text-zinc-50 flex flex-col items-center p-6 sm:p-12 font-sans">
    <div class="w-full max-w-lg flex flex-col gap-8">
        <header class="flex flex-col gap-2 border-b border-zinc-800 pb-6">
            <h1 class="text-3xl font-bold tracking-tight">Machine Setup</h1>
            <p class="text-zinc-400">Connect this device to your network.</p>
        </header>
        <main>
            {@render children()}
        </main>
    </div>
</div>

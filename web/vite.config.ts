import adapter from '@sveltejs/adapter-static';
import { sveltekit } from '@sveltejs/kit/vite';
import tailwindcss from '@tailwindcss/vite';
import { defineConfig } from 'vitest/config';

export default defineConfig({
  plugins: [
    tailwindcss(),
    sveltekit({
      adapter: adapter({ pages: 'build', assets: 'build', fallback: 'index.html' })
    })
  ],
  server: {
    proxy: {
      '/api': {
        target: process.env.API_BASE_URL ?? 'http://localhost:3000',
        changeOrigin: true
      }
    }
  },
  test: {
    environment: 'node',
    include: ['src/**/*.test.ts']
  }
});

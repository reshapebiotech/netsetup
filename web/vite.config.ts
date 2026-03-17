import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vite';

export default defineConfig({
  plugins: [sveltekit()],
  server: {
    proxy: {
      '/api': {
        target: process.env.API_BASE_URL ?? 'http://localhost:3000',
        changeOrigin: true
      },
      '/branding': {
        target: process.env.API_BASE_URL ?? 'http://localhost:3000',
        changeOrigin: true
      },
      '/favicon.ico': {
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

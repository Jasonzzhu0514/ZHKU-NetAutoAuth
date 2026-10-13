import { defineConfig } from '@playwright/test';

export default defineConfig({
  testDir: './frontend/tests',
  fullyParallel: true,
  reporter: 'line',
  use: { channel: 'msedge', baseURL: 'http://127.0.0.1:1420' },
  webServer: {
    command: 'npm exec vite preview -- --host 127.0.0.1 --port 1420 --strictPort',
    url: 'http://127.0.0.1:1420',
    reuseExistingServer: false,
    timeout: 30000,
  },
});

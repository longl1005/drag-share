import { defineConfig } from 'vitest/config';
import vue from '@vitejs/plugin-vue';
export default defineConfig({plugins:[vue()],clearScreen:false,server:{port:1420,strictPort:true},test:{environment:'jsdom',include:['tests/**/*.test.ts']}});

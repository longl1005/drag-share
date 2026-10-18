import {defineConfig} from '@playwright/test';
export default defineConfig({testDir:'./tests/browser',use:{baseURL:'http://127.0.0.1:1420',viewport:{width:320,height:440},deviceScaleFactor:2},reporter:'list',webServer:{command:'npm run dev',url:'http://127.0.0.1:1420',reuseExistingServer:true,timeout:30000}});

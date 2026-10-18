<script setup lang="ts">
import {ref,onMounted,onBeforeUnmount,computed} from 'vue';
import {listen,type UnlistenFn} from '@tauri-apps/api/event';
import {getCurrentWebviewWindow} from '@tauri-apps/api/webviewWindow';
import {invoke} from '@tauri-apps/api/core';
import {bridge,native} from './bridge';
import {useShare} from './useShare';
import {insideDropZone} from './drop';
import type {AppError,DesktopBridge} from './types';
import StatusSection from './components/StatusSection.vue';
import ShareList from './components/ShareList.vue';
import DropZone from './components/DropZone.vue';
const selected=ref<DesktopBridge>(bridge);
const c=useShare({snapshot:()=>selected.value.snapshot(),setEnabled:x=>selected.value.setEnabled(x),chooseDirectory:()=>selected.value.chooseDirectory(),add:x=>selected.value.add(x),remove:x=>selected.value.remove(x),copy:x=>selected.value.copy(x),reveal:x=>selected.value.reveal(x),settings:()=>selected.value.settings(),hide:()=>selected.value.hide()});
const {data,loading,busy,nativeBusy,toast,canManage}=c;
const hot=ref(false),unlisteners:UnlistenFn[]=[];let disposed=false;
const unavailable=computed(()=>data.value.warnings.some(w=>w.includes('共享目录')));
async function attach<T>(event:string,handler:(payload:T)=>void){const off=await listen<T>(event,e=>handler(e.payload));if(disposed)off();else unlisteners.push(off);}
function key(e:KeyboardEvent){if(e.key==='Escape'&&!busy.value&&!nativeBusy.value){hot.value=false;void selected.value.hide();}if((e.metaKey||e.ctrlKey)&&e.key.toLowerCase()==='q'&&native){e.preventDefault();void invoke('request_quit');}}
onMounted(async()=>{
 document.addEventListener('keydown',key);
 if(!native){selected.value=(await import('./preview')).previewBridge();const theme=new URLSearchParams(location.search).get('theme');if(theme==='light'||theme==='dark')document.documentElement.dataset.theme=theme;}
 else{
  await Promise.all([attach('panel-open',()=>void c.refresh()),attach('refresh',()=>void c.refresh()),attach<boolean>('native-busy',v=>nativeBusy.value=v),attach<AppError>('operation-error',e=>c.notify(e.message,true)),attach('choose-directory',()=>void c.addDirectory())]);
  const off=await getCurrentWebviewWindow().onDragDropEvent(event=>{
   const payload=event.payload;
   if(payload.type==='enter'||payload.type==='over'){
    const rect=document.querySelector('.dropzone')?.getBoundingClientRect();hot.value=canManage.value&&!!rect&&insideDropZone(payload.position,rect,window.devicePixelRatio||1);
   }else if(payload.type==='drop'){const rect=document.querySelector('.dropzone')?.getBoundingClientRect();const accept=canManage.value&&!!rect&&insideDropZone(payload.position,rect,window.devicePixelRatio||1);hot.value=false;if(accept)void c.drop(payload.paths);}else hot.value=false;
  });if(disposed)off();else unlisteners.push(off);
  const warning=await invoke<string|null>('startup_warning');if(warning)c.notify(warning,true);
 }
 await c.refresh();
});
onBeforeUnmount(()=>{disposed=true;unlisteners.forEach(off=>off());document.removeEventListener('keydown',key);c.dispose();});
</script>
<template><main class="panel" :aria-busy="busy||nativeBusy" @contextmenu.prevent>
 <StatusSection :status="data.status" :address="data.address" :fallback="data.addressFallback" :busy="busy||nativeBusy" :loading="loading" @toggle="c.toggle" @copy="c.copy()" @refresh="c.refresh"/>
 <div v-if="data.warnings.length" class="warning-banner" role="alert"><span>{{data.warnings[0]}}</span><button @click="c.settings">系统设置 ↗</button></div>
 <ShareList :shares="data.shares" :loading="loading" :can-manage="canManage" :unavailable="unavailable" @add="c.addDirectory" @copy="c.copy" @reveal="c.reveal" @remove="c.remove"/>
 <DropZone :enabled="canManage" :hot="hot" @add="c.addDirectory"/>
 <footer class="panel-footer"><span>{{!native?'浏览器预览 · 不操作系统':'Control + Option + S'}}</span><button title="打开系统共享设置" aria-label="打开系统共享设置" :disabled="busy||nativeBusy" @click="c.settings"><svg width="13" height="13" viewBox="0 0 20 20" fill="none" stroke="currentColor" stroke-width="1.3" aria-hidden="true"><path d="m8 2-.5 2-2 1L3.5 5l-1 2 1.5 1.5v3L2.5 13l1 2 2-.1 2 1 .5 2h3l.5-2 2-1 2 .1 1-2-1.5-1.5v-3L17.5 7l-1-2-2 .1-2-1L11 2Z"/><circle cx="10" cy="10" r="3"/></svg></button></footer>
 <Transition name="toast"><div v-if="toast" class="toast-message" :class="{error:toast.error}" :role="toast.error?'alert':'status'"><span>{{toast.error?'!':'✓'}}</span><p>{{toast.message}}</p><button aria-label="关闭提示" @click="toast=null">×</button></div></Transition>
</main></template>

import {computed,ref} from 'vue';
import type {AppError,DesktopBridge,Snapshot} from './types';
export function useShare(bridge:DesktopBridge) {
 const data=ref<Snapshot>({status:'unknown',shares:[],address:'',addressFallback:false,warnings:[]});
 const loading=ref(true),busy=ref(false),nativeBusy=ref(false);
 const toast=ref<{message:string;error:boolean}|null>(null);
 let generation=0;let timer:ReturnType<typeof setTimeout>|undefined;
 const canManage=computed(()=>data.value.status==='enabled'&&!busy.value&&!nativeBusy.value&&!loading.value&&!data.value.warnings.some(w=>w.includes('共享目录')));
 function notify(message:string,error=false){toast.value={message,error};clearTimeout(timer);timer=setTimeout(()=>toast.value=null,error?6500:2600);}
 function error(e:unknown){const v=e as AppError;if(v?.code==='USER_CANCELED')return;notify(v?.message||'操作未完成，请重试',true);}
 async function refresh(){const current=++generation;loading.value=true;try{const next=await bridge.snapshot();if(current===generation)data.value=next;}catch(e){if(current===generation){data.value={...data.value,status:'unknown'};error(e);}}finally{if(current===generation)loading.value=false;}}
 async function action(fn:()=>Promise<void>){if(busy.value||nativeBusy.value)return;busy.value=true;toast.value=null;try{await fn();}catch(e){error(e);}finally{await refresh();busy.value=false;}}
 async function toggle(){if(data.value.status==='unknown'||loading.value)return;await action(async()=>{const r=await bridge.setEnabled(data.value.status!=='enabled');if(!r.canceled)notify('共享状态已更新');});}
 async function addDirectory(){if(!canManage.value)return;await action(async()=>{const path=await bridge.chooseDirectory();if(path){const r=await bridge.add(path);if(!r.canceled)notify('已添加共享目录');}});}
 async function drop(paths:string[]){if(!canManage.value)return;await action(async()=>{let added=0;for(const path of paths){const r=await bridge.add(path);if(r.canceled)break;added++;}if(added)notify(`已添加 ${added} 个共享目录`);});}
 async function remove(id:string){if(!canManage.value)return;await action(async()=>{const r=await bridge.remove(id);if(!r.canceled)notify('已取消目录共享');});}
 async function copy(id?:string){if(!data.value.address||busy.value||nativeBusy.value)return;try{await bridge.copy(id);notify('已复制到剪贴板');}catch(e){error(e);}}
 async function reveal(id:string){try{await bridge.reveal(id);}catch(e){error(e);}}
 async function settings(){try{await bridge.settings();}catch(e){error(e);}}
 function dispose(){clearTimeout(timer);generation++;}
 return {data,loading,busy,nativeBusy,toast,canManage,refresh,toggle,addDirectory,drop,remove,copy,reveal,settings,notify,dispose};
}

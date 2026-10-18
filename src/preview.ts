// Browser-only preview. This module is imported only when no Tauri bridge exists.
import type {DesktopBridge,Snapshot} from './types';
export function previewBridge():DesktopBridge {
 const mode=new URLSearchParams(location.search).get('preview')||'normal';
 const state:Snapshot={status:mode==='off'?'disabled':mode==='error'?'unknown':'enabled',address:'smb://192.168.31.210',addressFallback:false,warnings:mode==='error'?['无法读取系统共享目录，请检查系统共享设置']:[],shares:mode==='empty'||mode==='error'?[]:[{id:'movies',name:'movies',path:'/Users/demo/Movies',readOnly:false,guestAccess:true,exists:true},{id:'Public',name:'Public',path:'/Users/demo/Public',readOnly:false,guestAccess:true,exists:true},{id:'Documents',name:'Documents',path:'/Users/demo/Documents',readOnly:true,guestAccess:false,exists:true}]};
 return {snapshot:async()=>{if(mode==='loading')await new Promise(r=>setTimeout(r,60000));return structuredClone(state);},setEnabled:async enabled=>{state.status=enabled?'enabled':'disabled';return{canceled:false};},chooseDirectory:async()=>null,add:async()=>({canceled:true}),remove:async()=>({canceled:true}),copy:async()=>{},reveal:async()=>{},settings:async()=>{},hide:async()=>{}};
}

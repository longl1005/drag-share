<script setup lang="ts">
import {ref,onMounted,onBeforeUnmount} from 'vue';
import FolderIcon from './FolderIcon.vue';
import type {ShareItem} from '../types';
defineProps<{shares:ShareItem[];loading:boolean;canManage:boolean;unavailable:boolean}>();
const emit=defineEmits<{add:[];copy:[id:string];reveal:[id:string];remove:[id:string]}>();
const active=ref<string|null>(null);
function close(){active.value=null;}
function action(type:'copy'|'reveal'|'remove',id:string){close();if(type==='copy')emit('copy',id);else if(type==='reveal')emit('reveal',id);else emit('remove',id);}
onMounted(()=>document.addEventListener('click',close));onBeforeUnmount(()=>document.removeEventListener('click',close));
</script>
<template><section class="list-section sec" aria-label="共享目录">
 <div class="list-head"><h2>共享目录 <span v-if="!loading">({{shares.length}})</span></h2><button class="add-btn" :disabled="!canManage" @click="$emit('add')">＋ 添加</button></div>
 <div v-if="loading" class="skeleton-list" aria-label="正在加载共享目录" aria-busy="true"><div v-for="i in 3" :key="i" class="skeleton-row"><span class="skeleton skeleton-icon"></span><span class="skeleton skeleton-name" :style="{width:`${100-i*13}px`}"></span><span class="skeleton skeleton-badge"></span></div></div>
 <div v-else-if="unavailable" class="empty"><FolderIcon large/><h3>暂时无法读取共享目录</h3><p>请检查系统共享设置后重试</p></div>
 <div v-else-if="!shares.length" class="empty"><FolderIcon large/><h3>还没有共享目录</h3><p>选择文件夹，即可在局域网内访问</p><button class="primary-btn empty-add" :disabled="!canManage" @click="$emit('add')">选择目录共享</button></div>
 <ul v-else class="share-scroll" :class="{muted:!canManage}"><li v-for="(share,index) in shares" :key="share.id" class="share-item" :title="share.path" @dblclick="canManage&&$emit('copy',share.id)">
  <FolderIcon/><span class="share-name">{{share.name}}</span><span v-if="!share.exists" class="missing" title="原目录已不存在" aria-label="原目录已不存在">⚠</span>
  <span class="badge">{{share.readOnly?'只读':'读写'}}{{share.guestAccess?'·访客':''}}</span>
  <button class="action-btn" :aria-label="`${share.name} 的操作`" aria-haspopup="menu" :aria-expanded="active===share.id" :disabled="!canManage" @click.stop="active=active===share.id?null:share.id" @keydown.esc.stop="close">···</button>
  <div v-if="active===share.id&&canManage" class="popover-menu" :class="{up:index>1}" role="menu" @click.stop @keydown.esc.stop="close">
   <button role="menuitem" @click="action('copy',share.id)">复制目录链接</button><button role="menuitem" :disabled="!share.exists" @click="action('reveal',share.id)">在访达中显示</button><div class="menu-divider"></div><button role="menuitem" class="danger" @click="action('remove',share.id)">取消共享</button>
  </div>
 </li></ul>
</section></template>

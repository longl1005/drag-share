<script setup lang="ts">
import type {ServiceStatus} from '../types';
defineProps<{status:ServiceStatus;address:string;fallback:boolean;busy:boolean;loading:boolean}>();
defineEmits<{toggle:[];copy:[];refresh:[]}>();
</script>
<template><section class="status-section sec" aria-label="共享服务状态">
 <div class="status-row"><h1><span class="dot" :class="status"></span>{{loading&&status==='unknown'?'正在读取状态':status==='enabled'?'共享已开启':status==='disabled'?'共享已关闭':'状态待确认'}}</h1>
 <button class="switch" :class="{off:status!=='enabled'}" role="switch" :aria-checked="status==='enabled'" :aria-label="status==='enabled'?'关闭共享服务':'开启共享服务'" :disabled="busy||loading||status==='unknown'" @click="$emit('toggle')"></button></div>
 <div class="addr-row"><span class="addr mono" :title="address">{{address||'正在获取本机地址…'}}</span><button class="copy-btn" :disabled="!address||busy" @click="$emit('copy')">复制</button></div>
 <p v-if="fallback&&address" class="network-note">未获取局域网 IP，使用本机 Bonjour 地址</p>
 <button v-if="status==='unknown'" class="primary-btn" :disabled="busy||loading" @click="$emit('refresh')"><span v-if="loading" class="spinner"></span>{{loading?'正在查询…':'重新读取状态'}}</button>
 <button v-else class="primary-btn" :class="{'btn-off':status==='disabled'}" :disabled="busy||loading" @click="$emit('toggle')"><span v-if="busy" class="spinner"></span>{{busy?'正在处理，请稍候…':status==='enabled'?'关闭共享':'开启共享'}}</button>
</section></template>

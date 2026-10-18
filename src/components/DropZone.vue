<script setup lang="ts">
defineProps<{enabled:boolean;hot:boolean}>();defineEmits<{add:[]}>();
</script>
<template><section class="drop-section sec"><button class="dropzone" :class="{hot,disabled:!enabled}" :disabled="!enabled" @click="$emit('add')"><svg width="19" height="19" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.4" aria-hidden="true"><path d="M12 3v12m-4-4 4 4 4-4M4 15v5h16v-5" stroke-linecap="round" stroke-linejoin="round"/></svg><span>{{hot?'松开以共享此文件夹':'拖入文件夹到此处共享'}}</span><small>{{enabled?'或点击选择文件夹':'开启共享后即可添加目录'}}</small></button></section></template>

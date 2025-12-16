<script setup lang="ts">
import { onMounted } from 'vue'
import { useAuthStore } from './stores/auth'
import { usePlayerStore } from './stores/player'
import { useLibraryStore } from './stores/library'
import { useKeyboardShortcuts } from './composables/useKeyboardShortcuts'

const authStore = useAuthStore()
const playerStore = usePlayerStore()
const libraryStore = useLibraryStore()

// 注册全局快捷键
useKeyboardShortcuts()

onMounted(async () => {
  // 初始化认证状态
  authStore.loadFromStorage()
  
  // 如果已登录，初始化播放器和加载收藏
  if (authStore.isAuthenticated) {
    playerStore.initAudio()
    await playerStore.restorePlayQueue()
    await libraryStore.fetchStarred()
  }
})
</script>

<template>
  <router-view />
</template>

<style>
/* 全局样式在 global.css 中定义 */
</style>

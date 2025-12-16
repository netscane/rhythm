<script setup lang="ts">
import { useRoute, useRouter } from 'vue-router'
import { useAuthStore } from '../stores/auth'
import { useLibraryStore } from '../stores/library'

defineProps<{
  collapsed: boolean
}>()

const emit = defineEmits<{
  toggle: []
}>()

const route = useRoute()
const router = useRouter()
const authStore = useAuthStore()
const libraryStore = useLibraryStore()

const menuItems = [
  { icon: '🏠', label: '首页', route: '/' },
  { icon: '🎤', label: '艺术家', route: '/artists' },
  { icon: '💿', label: '专辑', route: '/albums' },
  { icon: '📋', label: '播放列表', route: '/playlists' },
  { icon: '🎵', label: '流派', route: '/genres' },
  { icon: '⭐', label: '收藏', route: '/starred' },
  { icon: '⚙️', label: '设置', route: '/settings' }
]

const isActive = (path: string) => {
  if (path === '/') {
    return route.path === '/'
  }
  return route.path.startsWith(path)
}

function navigate(path: string) {
  router.push(path)
}

function logout() {
  authStore.logout()
  libraryStore.clearCache()
  router.push('/login')
}
</script>

<template>
  <aside class="sidebar" :class="{ collapsed }">
    <div class="sidebar-header">
      <div class="logo" @click="navigate('/')">
        <span class="logo-icon">🎵</span>
        <span v-if="!collapsed" class="logo-text">Rhythm</span>
      </div>
      <button class="toggle-btn" @click="emit('toggle')">
        {{ collapsed ? '→' : '←' }}
      </button>
    </div>

    <nav class="sidebar-nav">
      <ul class="nav-list">
        <li
          v-for="item in menuItems"
          :key="item.route"
          class="nav-item"
          :class="{ active: isActive(item.route) }"
          @click="navigate(item.route)"
        >
          <span class="nav-icon">{{ item.icon }}</span>
          <span v-if="!collapsed" class="nav-label">{{ item.label }}</span>
        </li>
      </ul>
    </nav>

    <div class="sidebar-footer">
      <div class="user-info" v-if="!collapsed">
        <span class="username">{{ authStore.username }}</span>
        <span class="server">{{ authStore.server }}</span>
      </div>
      <button class="logout-btn" @click="logout" :title="collapsed ? '退出登录' : ''">
        <span class="logout-icon">🚪</span>
        <span v-if="!collapsed">退出</span>
      </button>
    </div>
  </aside>
</template>

<style scoped>
.sidebar {
  position: fixed;
  left: 0;
  top: 0;
  bottom: 90px;
  width: 240px;
  background: var(--bg-secondary);
  border-right: 1px solid var(--border-color);
  display: flex;
  flex-direction: column;
  transition: width 0.3s ease;
  z-index: 100;
}

.sidebar.collapsed {
  width: 72px;
}

.sidebar-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 16px;
  border-bottom: 1px solid var(--border-color);
}

.logo {
  display: flex;
  align-items: center;
  gap: 8px;
  cursor: pointer;
}

.logo-icon {
  font-size: 24px;
}

.logo-text {
  font-size: 20px;
  font-weight: 700;
  color: var(--text-primary);
}

.toggle-btn {
  background: none;
  border: none;
  color: var(--text-secondary);
  cursor: pointer;
  padding: 4px 8px;
  border-radius: 4px;
  transition: background 0.2s;
}

.toggle-btn:hover {
  background: var(--bg-hover);
}

.sidebar-nav {
  flex: 1;
  overflow-y: auto;
  padding: 8px 0;
}

.nav-list {
  list-style: none;
  padding: 0;
  margin: 0;
}

.nav-item {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 12px 16px;
  cursor: pointer;
  color: var(--text-secondary);
  transition: all 0.2s;
}

.nav-item:hover {
  background: var(--bg-hover);
  color: var(--text-primary);
}

.nav-item.active {
  background: var(--accent-bg);
  color: var(--accent-color);
}

.nav-icon {
  font-size: 20px;
  width: 24px;
  text-align: center;
}

.nav-label {
  font-size: 14px;
  font-weight: 500;
}

.sidebar-footer {
  padding: 16px;
  border-top: 1px solid var(--border-color);
}

.user-info {
  display: flex;
  flex-direction: column;
  margin-bottom: 12px;
}

.username {
  font-size: 14px;
  font-weight: 600;
  color: var(--text-primary);
}

.server {
  font-size: 12px;
  color: var(--text-tertiary);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.logout-btn {
  display: flex;
  align-items: center;
  gap: 8px;
  width: 100%;
  padding: 8px 12px;
  background: none;
  border: 1px solid var(--border-color);
  border-radius: 6px;
  color: var(--text-secondary);
  cursor: pointer;
  transition: all 0.2s;
}

.logout-btn:hover {
  background: var(--bg-hover);
  color: var(--text-primary);
}

.collapsed .sidebar-header {
  justify-content: center;
  padding: 16px 8px;
}

.collapsed .toggle-btn {
  display: none;
}

.collapsed .nav-item {
  justify-content: center;
  padding: 12px;
}

.collapsed .logout-btn {
  justify-content: center;
}
</style>

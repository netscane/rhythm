<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useRouter } from 'vue-router'
import { useLibraryStore } from '../stores/library'
import subsonicApi from '../api/subsonic'

const router = useRouter()
const libraryStore = useLibraryStore()

const showCreateModal = ref(false)
const newPlaylistName = ref('')
const isCreating = ref(false)

onMounted(() => {
  libraryStore.fetchPlaylists()
})

function navigateToPlaylist(id: string) {
  router.push(`/playlist/${id}`)
}

function getCoverUrl(coverArt?: string) {
  if (coverArt) {
    return subsonicApi.getCoverArtUrl(coverArt, 150)
  }
  return ''
}

function formatDuration(seconds: number): string {
  const hours = Math.floor(seconds / 3600)
  const mins = Math.floor((seconds % 3600) / 60)
  if (hours > 0) {
    return `${hours}小时${mins}分钟`
  }
  return `${mins}分钟`
}

async function createPlaylist() {
  if (!newPlaylistName.value.trim()) return
  
  isCreating.value = true
  try {
    await libraryStore.createPlaylist(newPlaylistName.value.trim())
    newPlaylistName.value = ''
    showCreateModal.value = false
  } catch (e) {
    console.error('Failed to create playlist:', e)
  } finally {
    isCreating.value = false
  }
}
</script>

<template>
  <div class="playlists-view">
    <header class="page-header">
      <h1>播放列表</h1>
      <button class="create-btn" @click="showCreateModal = true">
        ➕ 新建播放列表
      </button>
    </header>

    <div v-if="libraryStore.isLoadingPlaylists" class="loading">
      加载中...
    </div>

    <div v-else-if="libraryStore.playlists.length === 0" class="empty">
      <p>暂无播放列表</p>
      <button class="action-btn" @click="showCreateModal = true">创建第一个播放列表</button>
    </div>

    <div v-else class="playlists-grid">
      <div
        v-for="playlist in libraryStore.playlists"
        :key="playlist.id"
        class="playlist-card"
        @click="navigateToPlaylist(playlist.id)"
      >
        <div class="playlist-cover">
          <img v-if="getCoverUrl(playlist.coverArt)" :src="getCoverUrl(playlist.coverArt)" />
          <span v-else class="placeholder">📋</span>
        </div>
        <div class="playlist-info">
          <div class="playlist-name">{{ playlist.name }}</div>
          <div class="playlist-meta">
            {{ playlist.songCount }} 首 • {{ formatDuration(playlist.duration) }}
          </div>
        </div>
      </div>
    </div>

    <!-- 创建播放列表弹窗 -->
    <div class="modal-overlay" v-if="showCreateModal" @click.self="showCreateModal = false">
      <div class="modal">
        <h2>新建播放列表</h2>
        <input
          v-model="newPlaylistName"
          type="text"
          placeholder="播放列表名称"
          @keyup.enter="createPlaylist"
        />
        <div class="modal-actions">
          <button class="cancel-btn" @click="showCreateModal = false">取消</button>
          <button class="confirm-btn" @click="createPlaylist" :disabled="isCreating || !newPlaylistName.trim()">
            {{ isCreating ? '创建中...' : '创建' }}
          </button>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.playlists-view {
  max-width: 1400px;
  margin: 0 auto;
}

.page-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 24px;
}

.page-header h1 {
  font-size: 32px;
  font-weight: 700;
  color: var(--text-primary);
  margin: 0;
}

.create-btn {
  padding: 10px 20px;
  background: var(--accent-color);
  border: none;
  border-radius: 20px;
  font-size: 14px;
  font-weight: 600;
  color: white;
  cursor: pointer;
  transition: all 0.2s;
}

.create-btn:hover {
  filter: brightness(1.1);
}

.playlists-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(200px, 1fr));
  gap: 20px;
}

.playlist-card {
  background: var(--bg-secondary);
  border-radius: 8px;
  padding: 16px;
  cursor: pointer;
  transition: all 0.2s;
}

.playlist-card:hover {
  background: var(--bg-hover);
  transform: translateY(-4px);
}

.playlist-cover {
  width: 100%;
  aspect-ratio: 1;
  border-radius: 4px;
  overflow: hidden;
  background: var(--bg-tertiary);
  display: flex;
  align-items: center;
  justify-content: center;
  margin-bottom: 12px;
}

.playlist-cover img {
  width: 100%;
  height: 100%;
  object-fit: cover;
}

.playlist-cover .placeholder {
  font-size: 48px;
}

.playlist-name {
  font-size: 14px;
  font-weight: 600;
  color: var(--text-primary);
  margin-bottom: 4px;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.playlist-meta {
  font-size: 12px;
  color: var(--text-tertiary);
}

.loading,
.empty {
  text-align: center;
  padding: 60px 20px;
  color: var(--text-tertiary);
}

.empty p {
  margin-bottom: 16px;
}

.action-btn {
  padding: 10px 20px;
  background: var(--bg-secondary);
  border: 1px solid var(--border-color);
  border-radius: 20px;
  font-size: 14px;
  color: var(--text-secondary);
  cursor: pointer;
}

.action-btn:hover {
  background: var(--bg-hover);
  color: var(--text-primary);
}

.modal-overlay {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.6);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 1000;
}

.modal {
  background: var(--bg-secondary);
  border-radius: 12px;
  padding: 24px;
  width: 100%;
  max-width: 400px;
}

.modal h2 {
  font-size: 20px;
  font-weight: 600;
  color: var(--text-primary);
  margin: 0 0 20px;
}

.modal input {
  width: 100%;
  padding: 12px 16px;
  background: var(--bg-tertiary);
  border: 1px solid var(--border-color);
  border-radius: 8px;
  font-size: 14px;
  color: var(--text-primary);
  margin-bottom: 20px;
}

.modal input:focus {
  outline: none;
  border-color: var(--accent-color);
}

.modal-actions {
  display: flex;
  justify-content: flex-end;
  gap: 12px;
}

.cancel-btn,
.confirm-btn {
  padding: 10px 20px;
  border-radius: 8px;
  font-size: 14px;
  font-weight: 500;
  cursor: pointer;
  transition: all 0.2s;
}

.cancel-btn {
  background: none;
  border: 1px solid var(--border-color);
  color: var(--text-secondary);
}

.cancel-btn:hover {
  background: var(--bg-hover);
}

.confirm-btn {
  background: var(--accent-color);
  border: none;
  color: white;
}

.confirm-btn:hover:not(:disabled) {
  filter: brightness(1.1);
}

.confirm-btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}
</style>

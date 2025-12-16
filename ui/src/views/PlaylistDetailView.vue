<script setup lang="ts">
import { ref, onMounted, watch, computed } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { useLibraryStore } from '../stores/library'
import { usePlayerStore } from '../stores/player'
import SongList from '../components/SongList.vue'
import subsonicApi, { type PlaylistWithSongs } from '../api/subsonic'

const route = useRoute()
const router = useRouter()
const libraryStore = useLibraryStore()
const playerStore = usePlayerStore()

const playlist = ref<PlaylistWithSongs | null>(null)
const isLoading = ref(true)
const showDeleteConfirm = ref(false)

async function loadPlaylist(id: string) {
  isLoading.value = true
  
  try {
    playlist.value = await libraryStore.fetchPlaylist(id)
  } catch (e) {
    console.error('Failed to load playlist:', e)
  } finally {
    isLoading.value = false
  }
}

onMounted(() => {
  loadPlaylist(route.params.id as string)
})

watch(() => route.params.id, (newId) => {
  if (newId) {
    loadPlaylist(newId as string)
  }
})

const coverUrl = computed(() => {
  if (playlist.value?.coverArt) {
    return subsonicApi.getCoverArtUrl(playlist.value.coverArt, 300)
  }
  // 使用第一首歌的封面
  if (playlist.value?.entry?.[0]?.coverArt) {
    return subsonicApi.getCoverArtUrl(playlist.value.entry[0].coverArt, 300)
  }
  return ''
})

const totalDuration = computed(() => {
  if (!playlist.value) return ''
  const total = playlist.value.duration
  const hours = Math.floor(total / 3600)
  const mins = Math.floor((total % 3600) / 60)
  if (hours > 0) {
    return `${hours} 小时 ${mins} 分钟`
  }
  return `${mins} 分钟`
})

function playPlaylist() {
  if (playlist.value?.entry) {
    playerStore.setQueueAndPlay(playlist.value.entry)
  }
}

function shufflePlaylist() {
  if (playlist.value?.entry) {
    const shuffled = [...playlist.value.entry].sort(() => Math.random() - 0.5)
    playerStore.setQueueAndPlay(shuffled)
  }
}

async function deletePlaylist() {
  if (!playlist.value) return
  
  const success = await libraryStore.deletePlaylist(playlist.value.id)
  if (success) {
    router.push('/playlists')
  }
}
</script>

<template>
  <div class="playlist-detail-view" v-if="!isLoading && playlist">
    <header class="playlist-header">
      <div class="playlist-cover">
        <img v-if="coverUrl" :src="coverUrl" :alt="playlist.name" />
        <span v-else class="placeholder">📋</span>
      </div>
      
      <div class="playlist-info">
        <span class="playlist-type">播放列表</span>
        <h1>{{ playlist.name }}</h1>
        <p class="description" v-if="playlist.comment">{{ playlist.comment }}</p>
        <div class="playlist-meta">
          <span>{{ playlist.songCount }} 首歌曲</span>
          <span class="dot">•</span>
          <span>{{ totalDuration }}</span>
        </div>
        
        <div class="actions">
          <button class="action-btn primary" @click="playPlaylist" :disabled="!playlist.entry?.length">
            ▶️ 播放
          </button>
          <button class="action-btn" @click="shufflePlaylist" :disabled="!playlist.entry?.length">
            🔀 随机播放
          </button>
          <button class="action-btn danger" @click="showDeleteConfirm = true">
            🗑️ 删除
          </button>
        </div>
      </div>
    </header>

    <section class="songs-section">
      <SongList
        v-if="playlist.entry && playlist.entry.length > 0"
        :songs="playlist.entry"
        :show-album="true"
      />
      <div class="empty" v-else>播放列表为空</div>
    </section>

    <!-- 删除确认弹窗 -->
    <div class="modal-overlay" v-if="showDeleteConfirm" @click.self="showDeleteConfirm = false">
      <div class="modal">
        <h2>删除播放列表</h2>
        <p>确定要删除播放列表 "{{ playlist.name }}" 吗？此操作无法撤销。</p>
        <div class="modal-actions">
          <button class="cancel-btn" @click="showDeleteConfirm = false">取消</button>
          <button class="confirm-btn danger" @click="deletePlaylist">删除</button>
        </div>
      </div>
    </div>
  </div>

  <div class="loading" v-else-if="isLoading">
    加载中...
  </div>
</template>

<style scoped>
.playlist-detail-view {
  max-width: 1400px;
  margin: 0 auto;
}

.playlist-header {
  display: flex;
  gap: 32px;
  margin-bottom: 40px;
}

.playlist-cover {
  width: 220px;
  height: 220px;
  border-radius: 8px;
  overflow: hidden;
  flex-shrink: 0;
  background: var(--bg-secondary);
  display: flex;
  align-items: center;
  justify-content: center;
  box-shadow: 0 8px 24px rgba(0, 0, 0, 0.3);
}

.playlist-cover img {
  width: 100%;
  height: 100%;
  object-fit: cover;
}

.playlist-cover .placeholder {
  font-size: 80px;
}

.playlist-info {
  display: flex;
  flex-direction: column;
  justify-content: flex-end;
}

.playlist-type {
  font-size: 12px;
  font-weight: 600;
  text-transform: uppercase;
  color: var(--text-secondary);
  margin-bottom: 8px;
}

.playlist-info h1 {
  font-size: 48px;
  font-weight: 700;
  color: var(--text-primary);
  margin: 0 0 8px;
  line-height: 1.1;
}

.description {
  font-size: 14px;
  color: var(--text-secondary);
  margin: 0 0 8px;
}

.playlist-meta {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 14px;
  color: var(--text-secondary);
  margin-bottom: 24px;
}

.dot {
  opacity: 0.5;
}

.actions {
  display: flex;
  gap: 12px;
}

.action-btn {
  padding: 12px 24px;
  background: var(--bg-tertiary);
  border: none;
  border-radius: 24px;
  font-size: 14px;
  font-weight: 600;
  color: var(--text-primary);
  cursor: pointer;
  transition: all 0.2s;
}

.action-btn:hover:not(:disabled) {
  background: var(--bg-hover);
}

.action-btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.action-btn.primary {
  background: var(--accent-color);
  color: white;
}

.action-btn.primary:hover:not(:disabled) {
  filter: brightness(1.1);
}

.action-btn.danger:hover {
  background: rgba(239, 68, 68, 0.2);
  color: #ef4444;
}

.songs-section {
  background: var(--bg-secondary);
  border-radius: 8px;
  padding: 16px;
}

.loading,
.empty {
  text-align: center;
  padding: 40px;
  color: var(--text-tertiary);
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
  margin: 0 0 12px;
}

.modal p {
  font-size: 14px;
  color: var(--text-secondary);
  margin: 0 0 20px;
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

.confirm-btn.danger {
  background: #ef4444;
}

.confirm-btn:hover {
  filter: brightness(1.1);
}
</style>

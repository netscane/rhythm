<script setup lang="ts">
import { ref, onMounted, watch } from 'vue'
import { useRoute } from 'vue-router'
import { useLibraryStore } from '../stores/library'
import { usePlayerStore } from '../stores/player'
import SongList from '../components/SongList.vue'
import type { Song } from '../api/subsonic'

const route = useRoute()
const libraryStore = useLibraryStore()
const playerStore = usePlayerStore()

const genreName = ref('')
const songs = ref<Song[]>([])
const isLoading = ref(true)
const offset = ref(0)
const hasMore = ref(true)
const pageSize = 50

async function loadSongs(reset = false) {
  if (reset) {
    offset.value = 0
    hasMore.value = true
    songs.value = []
  }
  
  isLoading.value = true
  
  try {
    const newSongs = await libraryStore.fetchSongsByGenre(genreName.value, pageSize, offset.value)
    
    if (reset) {
      songs.value = newSongs
    } else {
      songs.value.push(...newSongs)
    }
    
    hasMore.value = newSongs.length >= pageSize
    offset.value = songs.value.length
  } catch (e) {
    console.error('Failed to load songs:', e)
  } finally {
    isLoading.value = false
  }
}

onMounted(() => {
  genreName.value = decodeURIComponent(route.params.name as string)
  loadSongs(true)
})

watch(() => route.params.name, (newName) => {
  if (newName) {
    genreName.value = decodeURIComponent(newName as string)
    loadSongs(true)
  }
})

function playAll() {
  if (songs.value.length > 0) {
    playerStore.setQueueAndPlay(songs.value)
  }
}

function shufflePlay() {
  if (songs.value.length > 0) {
    const shuffled = [...songs.value].sort(() => Math.random() - 0.5)
    playerStore.setQueueAndPlay(shuffled)
  }
}

function loadMore() {
  if (!isLoading.value && hasMore.value) {
    loadSongs()
  }
}
</script>

<template>
  <div class="genre-detail-view">
    <header class="page-header">
      <div class="header-content">
        <div class="genre-icon">🎵</div>
        <div class="genre-info">
          <span class="genre-type">流派</span>
          <h1>{{ genreName }}</h1>
        </div>
      </div>
      
      <div class="actions">
        <button class="action-btn primary" @click="playAll" :disabled="songs.length === 0">
          ▶️ 播放全部
        </button>
        <button class="action-btn" @click="shufflePlay" :disabled="songs.length === 0">
          🔀 随机播放
        </button>
      </div>
    </header>

    <section class="songs-section">
      <SongList
        v-if="songs.length > 0"
        :songs="songs"
        :show-album="true"
      />
      
      <div class="loading" v-if="isLoading && songs.length === 0">
        加载中...
      </div>
      
      <div class="empty" v-if="!isLoading && songs.length === 0">
        该流派暂无歌曲
      </div>

      <div class="load-more" v-if="hasMore && songs.length > 0">
        <button
          class="load-more-btn"
          @click="loadMore"
          :disabled="isLoading"
        >
          {{ isLoading ? '加载中...' : '加载更多' }}
        </button>
      </div>
    </section>
  </div>
</template>

<style scoped>
.genre-detail-view {
  max-width: 1400px;
  margin: 0 auto;
}

.page-header {
  display: flex;
  align-items: flex-end;
  justify-content: space-between;
  margin-bottom: 32px;
  padding-bottom: 24px;
  border-bottom: 1px solid var(--border-color);
}

.header-content {
  display: flex;
  align-items: center;
  gap: 24px;
}

.genre-icon {
  width: 120px;
  height: 120px;
  border-radius: 16px;
  background: linear-gradient(135deg, var(--accent-color), #8b5cf6);
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 48px;
  box-shadow: 0 8px 24px rgba(0, 0, 0, 0.3);
}

.genre-info {
  display: flex;
  flex-direction: column;
}

.genre-type {
  font-size: 12px;
  font-weight: 600;
  text-transform: uppercase;
  color: var(--text-secondary);
  margin-bottom: 8px;
}

.genre-info h1 {
  font-size: 48px;
  font-weight: 700;
  color: var(--text-primary);
  margin: 0;
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

.load-more {
  display: flex;
  justify-content: center;
  padding: 24px;
}

.load-more-btn {
  padding: 12px 32px;
  background: var(--bg-tertiary);
  border: none;
  border-radius: 24px;
  font-size: 14px;
  color: var(--text-secondary);
  cursor: pointer;
  transition: all 0.2s;
}

.load-more-btn:hover:not(:disabled) {
  background: var(--bg-hover);
  color: var(--text-primary);
}

.load-more-btn:disabled {
  opacity: 0.6;
  cursor: not-allowed;
}
</style>

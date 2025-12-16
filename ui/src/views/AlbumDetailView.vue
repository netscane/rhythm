<script setup lang="ts">
import { ref, onMounted, watch, computed } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { useLibraryStore } from '../stores/library'
import { usePlayerStore } from '../stores/player'
import SongList from '../components/SongList.vue'
import subsonicApi, { type AlbumWithSongs } from '../api/subsonic'

const route = useRoute()
const router = useRouter()
const libraryStore = useLibraryStore()
const playerStore = usePlayerStore()

const album = ref<AlbumWithSongs | null>(null)
const isLoading = ref(true)

async function loadAlbum(id: string) {
  isLoading.value = true
  
  try {
    album.value = await libraryStore.fetchAlbum(id)
  } catch (e) {
    console.error('Failed to load album:', e)
  } finally {
    isLoading.value = false
  }
}

onMounted(() => {
  loadAlbum(route.params.id as string)
})

watch(() => route.params.id, (newId) => {
  if (newId) {
    loadAlbum(newId as string)
  }
})

const coverUrl = computed(() => {
  if (album.value?.coverArt) {
    return subsonicApi.getCoverArtUrl(album.value.coverArt, 300)
  }
  return ''
})

const totalDuration = computed(() => {
  if (!album.value?.song) return ''
  const total = album.value.song.reduce((acc, s) => acc + (s.duration || 0), 0)
  const hours = Math.floor(total / 3600)
  const mins = Math.floor((total % 3600) / 60)
  if (hours > 0) {
    return `${hours} 小时 ${mins} 分钟`
  }
  return `${mins} 分钟`
})

const isStarred = computed(() => {
  if (!album.value) return false
  return libraryStore.isStarred(album.value.id, 'album')
})

function playAlbum() {
  if (album.value?.song) {
    playerStore.setQueueAndPlay(album.value.song)
  }
}

function shuffleAlbum() {
  if (album.value?.song) {
    const shuffled = [...album.value.song].sort(() => Math.random() - 0.5)
    playerStore.setQueueAndPlay(shuffled)
  }
}

function goToArtist() {
  if (album.value?.artistId) {
    router.push(`/artist/${album.value.artistId}`)
  }
}

async function toggleStar() {
  if (album.value) {
    await libraryStore.toggleStar(album.value.id, 'album')
  }
}
</script>

<template>
  <div class="album-detail-view" v-if="!isLoading && album">
    <header class="album-header">
      <div class="album-cover">
        <img v-if="coverUrl" :src="coverUrl" :alt="album.name" />
        <span v-else class="placeholder">💿</span>
      </div>
      
      <div class="album-info">
        <span class="album-type">专辑</span>
        <h1>{{ album.name }}</h1>
        <div class="album-meta">
          <span class="artist" @click="goToArtist">{{ album.artist }}</span>
          <span class="dot">•</span>
          <span v-if="album.year">{{ album.year }}</span>
          <span class="dot" v-if="album.year">•</span>
          <span>{{ album.songCount || album.song?.length || 0 }} 首歌曲</span>
          <span class="dot">•</span>
          <span>{{ totalDuration }}</span>
        </div>
        
        <div class="actions">
          <button class="action-btn primary" @click="playAlbum">
            ▶️ 播放
          </button>
          <button class="action-btn" @click="shuffleAlbum">
            🔀 随机播放
          </button>
          <button class="action-btn" :class="{ starred: isStarred }" @click="toggleStar">
            {{ isStarred ? '❤️ 已收藏' : '🤍 收藏' }}
          </button>
        </div>
      </div>
    </header>

    <section class="songs-section">
      <SongList
        v-if="album.song && album.song.length > 0"
        :songs="album.song"
        :show-album="false"
        :show-track-number="true"
      />
      <div class="empty" v-else>暂无歌曲</div>
    </section>
  </div>

  <div class="loading" v-else-if="isLoading">
    加载中...
  </div>
</template>

<style scoped>
.album-detail-view {
  max-width: 1400px;
  margin: 0 auto;
}

.album-header {
  display: flex;
  gap: 32px;
  margin-bottom: 40px;
}

.album-cover {
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

.album-cover img {
  width: 100%;
  height: 100%;
  object-fit: cover;
}

.album-cover .placeholder {
  font-size: 80px;
}

.album-info {
  display: flex;
  flex-direction: column;
  justify-content: flex-end;
}

.album-type {
  font-size: 12px;
  font-weight: 600;
  text-transform: uppercase;
  color: var(--text-secondary);
  margin-bottom: 8px;
}

.album-info h1 {
  font-size: 48px;
  font-weight: 700;
  color: var(--text-primary);
  margin: 0 0 16px;
  line-height: 1.1;
}

.album-meta {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 14px;
  color: var(--text-secondary);
  margin-bottom: 24px;
}

.artist {
  font-weight: 600;
  color: var(--text-primary);
  cursor: pointer;
}

.artist:hover {
  text-decoration: underline;
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

.action-btn:hover {
  background: var(--bg-hover);
}

.action-btn.primary {
  background: var(--accent-color);
  color: white;
}

.action-btn.primary:hover {
  filter: brightness(1.1);
}

.action-btn.starred {
  background: var(--accent-color);
  color: white;
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
</style>

<script setup lang="ts">
import { ref, onMounted, watch, computed } from 'vue'
import { useRoute } from 'vue-router'
import { useLibraryStore } from '../stores/library'
import { usePlayerStore } from '../stores/player'
import AlbumCard from '../components/AlbumCard.vue'
import subsonicApi, { type ArtistWithAlbums, type ArtistInfo, type Song } from '../api/subsonic'

const route = useRoute()
const libraryStore = useLibraryStore()
const playerStore = usePlayerStore()

const artist = ref<ArtistWithAlbums | null>(null)
const artistInfo = ref<ArtistInfo | null>(null)
const topSongs = ref<Song[]>([])
const isLoading = ref(true)

async function loadArtist(id: string) {
  isLoading.value = true
  
  try {
    const [artistData, info, songs] = await Promise.all([
      libraryStore.fetchArtist(id),
      subsonicApi.getArtistInfo(id, 10).catch(() => null),
      subsonicApi.getTopSongs(id, 10).catch(() => [])
    ])
    
    artist.value = artistData
    artistInfo.value = info
    topSongs.value = songs
  } catch (e) {
    console.error('Failed to load artist:', e)
  } finally {
    isLoading.value = false
  }
}

onMounted(() => {
  loadArtist(route.params.id as string)
})

watch(() => route.params.id, (newId) => {
  if (newId) {
    loadArtist(newId as string)
  }
})

const isStarred = computed(() => {
  if (!artist.value) return false
  return libraryStore.isStarred(artist.value.id, 'artist')
})

function getCoverUrl(coverArt?: string, size = 300) {
  if (coverArt) {
    return subsonicApi.getCoverArtUrl(coverArt, size)
  }
  return ''
}

function playTopSongs() {
  if (topSongs.value.length > 0) {
    playerStore.setQueueAndPlay(topSongs.value)
  }
}

async function playAllAlbums() {
  if (!artist.value?.album) return
  
  const allSongs: Song[] = []
  for (const album of artist.value.album) {
    const albumData = await libraryStore.fetchAlbum(album.id)
    if (albumData?.song) {
      allSongs.push(...albumData.song)
    }
  }
  
  if (allSongs.length > 0) {
    playerStore.setQueueAndPlay(allSongs)
  }
}

async function toggleStar() {
  if (artist.value) {
    await libraryStore.toggleStar(artist.value.id, 'artist')
  }
}
</script>

<template>
  <div class="artist-detail-view" v-if="!isLoading && artist">
    <header class="artist-header">
      <div class="artist-image">
        <img
          v-if="artistInfo?.largeImageUrl || getCoverUrl(artist.coverArt)"
          :src="artistInfo?.largeImageUrl || getCoverUrl(artist.coverArt)"
          :alt="artist.name"
        />
        <span v-else class="placeholder">🎤</span>
      </div>
      
      <div class="artist-info">
        <h1>{{ artist.name }}</h1>
        <p class="stats">{{ artist.albumCount || artist.album?.length || 0 }} 张专辑</p>
        
        <div class="actions">
          <button class="action-btn primary" @click="playAllAlbums">
            ▶️ 播放全部
          </button>
          <button class="action-btn" @click="playTopSongs" v-if="topSongs.length > 0">
            🔥 热门歌曲
          </button>
          <button class="action-btn" :class="{ starred: isStarred }" @click="toggleStar">
            {{ isStarred ? '❤️ 已收藏' : '🤍 收藏' }}
          </button>
        </div>
      </div>
    </header>

    <section class="section" v-if="artistInfo?.biography">
      <h2>简介</h2>
      <p class="biography" v-html="artistInfo.biography"></p>
    </section>

    <section class="section" v-if="topSongs.length > 0">
      <h2>热门歌曲</h2>
      <div class="top-songs">
        <div
          v-for="(song, index) in topSongs.slice(0, 5)"
          :key="song.id"
          class="song-item"
          @click="playerStore.setQueueAndPlay(topSongs, index)"
        >
          <span class="song-index">{{ index + 1 }}</span>
          <div class="song-cover">
            <img v-if="getCoverUrl(song.coverArt, 50)" :src="getCoverUrl(song.coverArt, 50)" />
            <span v-else>🎵</span>
          </div>
          <div class="song-info">
            <div class="song-title">{{ song.title }}</div>
            <div class="song-album">{{ song.album }}</div>
          </div>
          <span class="song-duration">{{ formatDuration(song.duration) }}</span>
        </div>
      </div>
    </section>

    <section class="section">
      <h2>专辑</h2>
      <div class="albums-grid">
        <AlbumCard
          v-for="album in artist.album"
          :key="album.id"
          :album="album"
        />
      </div>
    </section>
  </div>

  <div class="loading" v-else-if="isLoading">
    加载中...
  </div>
</template>

<script lang="ts">
function formatDuration(seconds?: number): string {
  if (!seconds) return '0:00'
  const mins = Math.floor(seconds / 60)
  const secs = Math.floor(seconds % 60)
  return `${mins}:${secs.toString().padStart(2, '0')}`
}
</script>

<style scoped>
.artist-detail-view {
  max-width: 1400px;
  margin: 0 auto;
}

.artist-header {
  display: flex;
  gap: 32px;
  margin-bottom: 40px;
}

.artist-image {
  width: 220px;
  height: 220px;
  border-radius: 50%;
  overflow: hidden;
  flex-shrink: 0;
  background: var(--bg-secondary);
  display: flex;
  align-items: center;
  justify-content: center;
  box-shadow: 0 8px 24px rgba(0, 0, 0, 0.3);
}

.artist-image img {
  width: 100%;
  height: 100%;
  object-fit: cover;
}

.artist-image .placeholder {
  font-size: 80px;
}

.artist-info {
  display: flex;
  flex-direction: column;
  justify-content: center;
}

.artist-info h1 {
  font-size: 48px;
  font-weight: 700;
  color: var(--text-primary);
  margin: 0 0 8px;
}

.stats {
  font-size: 16px;
  color: var(--text-secondary);
  margin: 0 0 24px;
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

.section {
  margin-bottom: 40px;
}

.section h2 {
  font-size: 24px;
  font-weight: 700;
  color: var(--text-primary);
  margin: 0 0 20px;
}

.biography {
  font-size: 14px;
  line-height: 1.8;
  color: var(--text-secondary);
  max-width: 800px;
}

.top-songs {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.song-item {
  display: flex;
  align-items: center;
  gap: 16px;
  padding: 8px 12px;
  border-radius: 8px;
  cursor: pointer;
  transition: background 0.2s;
}

.song-item:hover {
  background: var(--bg-secondary);
}

.song-index {
  width: 24px;
  text-align: center;
  font-size: 14px;
  color: var(--text-tertiary);
}

.song-cover {
  width: 40px;
  height: 40px;
  border-radius: 4px;
  overflow: hidden;
  background: var(--bg-tertiary);
  display: flex;
  align-items: center;
  justify-content: center;
}

.song-cover img {
  width: 100%;
  height: 100%;
  object-fit: cover;
}

.song-info {
  flex: 1;
  min-width: 0;
}

.song-title {
  font-size: 14px;
  font-weight: 500;
  color: var(--text-primary);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.song-album {
  font-size: 12px;
  color: var(--text-tertiary);
}

.song-duration {
  font-size: 14px;
  color: var(--text-tertiary);
}

.albums-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(180px, 1fr));
  gap: 20px;
}

.loading {
  text-align: center;
  padding: 40px;
  color: var(--text-tertiary);
}
</style>

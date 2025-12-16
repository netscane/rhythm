<script setup lang="ts">
import { onMounted } from 'vue'
import { useRouter } from 'vue-router'
import { useLibraryStore } from '../stores/library'
import subsonicApi from '../api/subsonic'
import LazyImage from '../components/LazyImage.vue'

const router = useRouter()
const libraryStore = useLibraryStore()

onMounted(() => {
  if (libraryStore.artistIndexes.length === 0) {
    libraryStore.fetchArtists()
  }
})

function navigateToArtist(id: string) {
  router.push(`/artist/${id}`)
}

function getCoverUrl(coverArt?: string) {
  if (coverArt) {
    return subsonicApi.getCoverArtUrl(coverArt, 100)
  }
  return ''
}
</script>

<template>
  <div class="artists-view">
    <header class="page-header">
      <h1>艺术家</h1>
    </header>

    <div v-if="libraryStore.isLoadingArtists" class="loading">
      加载中...
    </div>

    <div v-else class="artists-content">
      <div
        v-for="index in libraryStore.artistIndexes"
        :key="index.name"
        class="artist-group"
      >
        <h2 class="group-letter">{{ index.name }}</h2>
        <div class="artists-grid">
          <div
            v-for="artist in index.artist"
            :key="artist.id"
            class="artist-card"
            @click="navigateToArtist(artist.id)"
          >
            <div class="artist-image">
              <LazyImage
                v-if="artist.coverArt"
                :src="getCoverUrl(artist.coverArt)"
                :alt="artist.name"
                placeholder="🎤"
              />
              <span v-else class="placeholder">🎤</span>
            </div>
            <div class="artist-info">
              <div class="artist-name">{{ artist.name }}</div>
              <div class="artist-albums" v-if="artist.albumCount">
                {{ artist.albumCount }} 张专辑
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.artists-view {
  max-width: 1400px;
  margin: 0 auto;
}

.page-header {
  margin-bottom: 24px;
}

.page-header h1 {
  font-size: 32px;
  font-weight: 700;
  color: var(--text-primary);
  margin: 0;
}

.loading {
  text-align: center;
  padding: 40px;
  color: var(--text-tertiary);
}

.artist-group {
  margin-bottom: 32px;
}

.group-letter {
  font-size: 20px;
  font-weight: 600;
  color: var(--text-secondary);
  margin: 0 0 16px;
  padding-bottom: 8px;
  border-bottom: 1px solid var(--border-color);
}

.artists-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(200px, 1fr));
  gap: 16px;
}

.artist-card {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 12px;
  background: var(--bg-secondary);
  border-radius: 8px;
  cursor: pointer;
  transition: all 0.2s;
}

.artist-card:hover {
  background: var(--bg-hover);
  transform: translateY(-2px);
}

.artist-image {
  width: 56px;
  height: 56px;
  border-radius: 50%;
  overflow: hidden;
  flex-shrink: 0;
  background: var(--bg-tertiary);
  display: flex;
  align-items: center;
  justify-content: center;
}

.artist-image img {
  width: 100%;
  height: 100%;
  object-fit: cover;
}

.artist-image .placeholder {
  font-size: 24px;
}

.artist-info {
  flex: 1;
  min-width: 0;
}

.artist-name {
  font-size: 14px;
  font-weight: 600;
  color: var(--text-primary);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.artist-albums {
  font-size: 12px;
  color: var(--text-tertiary);
  margin-top: 2px;
}
</style>

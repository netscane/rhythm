<script setup lang="ts">
import { ref, watch, onMounted } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { useLibraryStore } from '../stores/library'
import AlbumCard from '../components/AlbumCard.vue'
import SongList from '../components/SongList.vue'
import subsonicApi, { type SearchResult3 } from '../api/subsonic'

const route = useRoute()
const router = useRouter()
const libraryStore = useLibraryStore()

const query = ref('')
const results = ref<SearchResult3>({})
const isLoading = ref(false)

async function search(q: string) {
  if (!q.trim()) {
    results.value = {}
    return
  }

  isLoading.value = true
  
  try {
    results.value = await libraryStore.search(q, {
      artistCount: 10,
      albumCount: 12,
      songCount: 30
    })
  } catch (e) {
    console.error('Search failed:', e)
  } finally {
    isLoading.value = false
  }
}

onMounted(() => {
  const q = route.query.q as string
  if (q) {
    query.value = q
    search(q)
  }
})

watch(() => route.query.q, (newQuery) => {
  if (newQuery && typeof newQuery === 'string') {
    query.value = newQuery
    search(newQuery)
  }
})

function getCoverUrl(coverArt?: string) {
  if (coverArt) {
    return subsonicApi.getCoverArtUrl(coverArt, 100)
  }
  return ''
}

function navigateToArtist(id: string) {
  router.push(`/artist/${id}`)
}

const hasResults = () => {
  return (results.value.artist?.length || 0) > 0 ||
         (results.value.album?.length || 0) > 0 ||
         (results.value.song?.length || 0) > 0
}
</script>

<template>
  <div class="search-view">
    <div v-if="isLoading" class="loading">
      搜索中...
    </div>

    <div v-else-if="!hasResults() && query" class="empty">
      未找到 "{{ query }}" 的相关结果
    </div>

    <div v-else-if="hasResults()" class="search-results">
      <!-- 艺术家 -->
      <section v-if="results.artist?.length" class="section">
        <h2>艺术家</h2>
        <div class="artists-grid">
          <div
            v-for="artist in results.artist"
            :key="artist.id"
            class="artist-card"
            @click="navigateToArtist(artist.id)"
          >
            <div class="artist-image">
              <img v-if="getCoverUrl(artist.coverArt)" :src="getCoverUrl(artist.coverArt)" />
              <span v-else class="placeholder">🎤</span>
            </div>
            <div class="artist-info">
              <div class="artist-name">{{ artist.name }}</div>
            </div>
          </div>
        </div>
      </section>

      <!-- 专辑 -->
      <section v-if="results.album?.length" class="section">
        <h2>专辑</h2>
        <div class="albums-grid">
          <AlbumCard
            v-for="album in results.album"
            :key="album.id"
            :album="album"
          />
        </div>
      </section>

      <!-- 歌曲 -->
      <section v-if="results.song?.length" class="section">
        <h2>歌曲</h2>
        <SongList :songs="results.song" :show-album="true" />
      </section>
    </div>

    <div v-else class="hint">
      输入关键词开始搜索
    </div>
  </div>
</template>

<style scoped>
.search-view {
  max-width: 1400px;
  margin: 0 auto;
}

.section {
  margin-bottom: 40px;
}

.section h2 {
  font-size: 20px;
  font-weight: 700;
  color: var(--text-primary);
  margin: 0 0 16px;
}

.artists-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(180px, 1fr));
  gap: 16px;
}

.artist-card {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 12px;
  padding: 16px;
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
  width: 80px;
  height: 80px;
  border-radius: 50%;
  overflow: hidden;
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
  font-size: 32px;
}

.artist-name {
  font-size: 14px;
  font-weight: 600;
  color: var(--text-primary);
  text-align: center;
}

.albums-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(180px, 1fr));
  gap: 20px;
}

.loading,
.empty,
.hint {
  text-align: center;
  padding: 60px 20px;
  color: var(--text-tertiary);
  font-size: 16px;
}
</style>

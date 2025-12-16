<script setup lang="ts">
import { onMounted } from 'vue'
import { useRouter } from 'vue-router'
import { useLibraryStore } from '../stores/library'

const router = useRouter()
const libraryStore = useLibraryStore()

onMounted(() => {
  if (libraryStore.genres.length === 0) {
    libraryStore.fetchGenres()
  }
})

function navigateToGenre(name: string) {
  router.push(`/genre/${encodeURIComponent(name)}`)
}
</script>

<template>
  <div class="genres-view">
    <header class="page-header">
      <h1>流派</h1>
    </header>

    <div v-if="libraryStore.isLoadingGenres" class="loading">
      加载中...
    </div>

    <div v-else-if="libraryStore.genres.length === 0" class="empty">
      暂无流派信息
    </div>

    <div v-else class="genres-grid">
      <div
        v-for="genre in libraryStore.genres"
        :key="genre.value"
        class="genre-card"
        @click="navigateToGenre(genre.value)"
      >
        <div class="genre-icon">🎵</div>
        <div class="genre-info">
          <div class="genre-name">{{ genre.value }}</div>
          <div class="genre-stats">
            {{ genre.albumCount }} 张专辑 • {{ genre.songCount }} 首歌曲
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.genres-view {
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

.genres-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
  gap: 16px;
}

.genre-card {
  display: flex;
  align-items: center;
  gap: 16px;
  padding: 20px;
  background: var(--bg-secondary);
  border-radius: 12px;
  cursor: pointer;
  transition: all 0.2s;
}

.genre-card:hover {
  background: var(--bg-hover);
  transform: translateY(-2px);
}

.genre-icon {
  width: 56px;
  height: 56px;
  border-radius: 12px;
  background: linear-gradient(135deg, var(--accent-color), #8b5cf6);
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 24px;
  flex-shrink: 0;
}

.genre-info {
  flex: 1;
  min-width: 0;
}

.genre-name {
  font-size: 16px;
  font-weight: 600;
  color: var(--text-primary);
  margin-bottom: 4px;
}

.genre-stats {
  font-size: 13px;
  color: var(--text-tertiary);
}

.loading,
.empty {
  text-align: center;
  padding: 60px 20px;
  color: var(--text-tertiary);
}
</style>

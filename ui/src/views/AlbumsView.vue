<script setup lang="ts">
import { ref, onMounted, watch } from 'vue'
import { useLibraryStore } from '../stores/library'
import AlbumCard from '../components/AlbumCard.vue'

const libraryStore = useLibraryStore()

type SortType = 'newest' | 'alphabeticalByName' | 'alphabeticalByArtist' | 'random' | 'frequent' | 'recent' | 'highest'

const sortType = ref<SortType>('newest')
const sortOptions: { value: SortType; label: string }[] = [
  { value: 'newest', label: '最新添加' },
  { value: 'alphabeticalByName', label: '按名称' },
  { value: 'alphabeticalByArtist', label: '按艺术家' },
  { value: 'random', label: '随机' },
  { value: 'frequent', label: '最常播放' },
  { value: 'recent', label: '最近播放' },
  { value: 'highest', label: '评分最高' }
]

const offset = ref(0)
const hasMore = ref(true)
const pageSize = 50

async function loadAlbums(reset = false) {
  if (reset) {
    offset.value = 0
    hasMore.value = true
  }
  
  const prevCount = libraryStore.albums.length
  await libraryStore.fetchAlbums(sortType.value, { size: pageSize, offset: offset.value })
  
  const newCount = libraryStore.albums.length
  if (reset) {
    hasMore.value = newCount >= pageSize
  } else {
    hasMore.value = newCount - prevCount >= pageSize
  }
  
  offset.value = newCount
}

onMounted(() => {
  loadAlbums(true)
})

watch(sortType, () => {
  loadAlbums(true)
})

function loadMore() {
  if (!libraryStore.isLoadingAlbums && hasMore.value) {
    loadAlbums()
  }
}
</script>

<template>
  <div class="albums-view">
    <header class="page-header">
      <h1>专辑</h1>
      <select v-model="sortType" class="sort-select">
        <option v-for="opt in sortOptions" :key="opt.value" :value="opt.value">
          {{ opt.label }}
        </option>
      </select>
    </header>

    <div class="albums-grid">
      <AlbumCard
        v-for="album in libraryStore.albums"
        :key="album.id"
        :album="album"
      />
    </div>

    <div class="load-more" v-if="hasMore">
      <button
        class="load-more-btn"
        @click="loadMore"
        :disabled="libraryStore.isLoadingAlbums"
      >
        {{ libraryStore.isLoadingAlbums ? '加载中...' : '加载更多' }}
      </button>
    </div>

    <div class="loading" v-if="libraryStore.isLoadingAlbums && libraryStore.albums.length === 0">
      加载中...
    </div>
  </div>
</template>

<style scoped>
.albums-view {
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

.sort-select {
  padding: 8px 16px;
  background: var(--bg-secondary);
  border: 1px solid var(--border-color);
  border-radius: 8px;
  font-size: 14px;
  color: var(--text-primary);
  cursor: pointer;
}

.sort-select:focus {
  outline: none;
  border-color: var(--accent-color);
}

.albums-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(180px, 1fr));
  gap: 20px;
}

.load-more {
  display: flex;
  justify-content: center;
  padding: 32px;
}

.load-more-btn {
  padding: 12px 32px;
  background: var(--bg-secondary);
  border: 1px solid var(--border-color);
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

.loading {
  text-align: center;
  padding: 40px;
  color: var(--text-tertiary);
}
</style>

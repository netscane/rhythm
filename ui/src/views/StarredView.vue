<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useLibraryStore } from '../stores/library'
import { usePlayerStore } from '../stores/player'
import AlbumCard from '../components/AlbumCard.vue'
import SongList from '../components/SongList.vue'
import subsonicApi from '../api/subsonic'
import { useRouter } from 'vue-router'

const router = useRouter()
const libraryStore = useLibraryStore()
const playerStore = usePlayerStore()

type Tab = 'songs' | 'albums' | 'artists'
const activeTab = ref<Tab>('songs')

const tabs: { value: Tab; label: string }[] = [
  { value: 'songs', label: '歌曲' },
  { value: 'albums', label: '专辑' },
  { value: 'artists', label: '艺术家' }
]

onMounted(() => {
  libraryStore.fetchStarred()
})

function playAllSongs() {
  if (libraryStore.starred.song.length > 0) {
    playerStore.setQueueAndPlay(libraryStore.starred.song)
  }
}

function getCoverUrl(coverArt?: string) {
  if (coverArt) {
    return subsonicApi.getCoverArtUrl(coverArt, 100)
  }
  return ''
}

function navigateToArtist(id: string) {
  router.push(`/artist/${id}`)
}
</script>

<template>
  <div class="starred-view">
    <header class="page-header">
      <h1>收藏</h1>
      <button
        v-if="activeTab === 'songs' && libraryStore.starred.song.length > 0"
        class="play-btn"
        @click="playAllSongs"
      >
        ▶️ 播放全部
      </button>
    </header>

    <div class="tabs">
      <button
        v-for="tab in tabs"
        :key="tab.value"
        class="tab"
        :class="{ active: activeTab === tab.value }"
        @click="activeTab = tab.value"
      >
        {{ tab.label }}
        <span class="count">
          {{ tab.value === 'songs' ? libraryStore.starred.song.length :
             tab.value === 'albums' ? libraryStore.starred.album.length :
             libraryStore.starred.artist.length }}
        </span>
      </button>
    </div>

    <div class="tab-content">
      <!-- 歌曲 -->
      <div v-if="activeTab === 'songs'">
        <SongList
          v-if="libraryStore.starred.song.length > 0"
          :songs="libraryStore.starred.song"
          :show-album="true"
        />
        <div class="empty" v-else>暂无收藏的歌曲</div>
      </div>

      <!-- 专辑 -->
      <div v-if="activeTab === 'albums'">
        <div class="albums-grid" v-if="libraryStore.starred.album.length > 0">
          <AlbumCard
            v-for="album in libraryStore.starred.album"
            :key="album.id"
            :album="album"
          />
        </div>
        <div class="empty" v-else>暂无收藏的专辑</div>
      </div>

      <!-- 艺术家 -->
      <div v-if="activeTab === 'artists'">
        <div class="artists-grid" v-if="libraryStore.starred.artist.length > 0">
          <div
            v-for="artist in libraryStore.starred.artist"
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
              <div class="artist-albums" v-if="artist.albumCount">
                {{ artist.albumCount }} 张专辑
              </div>
            </div>
          </div>
        </div>
        <div class="empty" v-else>暂无收藏的艺术家</div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.starred-view {
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

.play-btn {
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

.play-btn:hover {
  filter: brightness(1.1);
}

.tabs {
  display: flex;
  gap: 8px;
  margin-bottom: 24px;
  border-bottom: 1px solid var(--border-color);
  padding-bottom: 16px;
}

.tab {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 16px;
  background: none;
  border: none;
  border-radius: 20px;
  font-size: 14px;
  font-weight: 500;
  color: var(--text-secondary);
  cursor: pointer;
  transition: all 0.2s;
}

.tab:hover {
  background: var(--bg-secondary);
  color: var(--text-primary);
}

.tab.active {
  background: var(--accent-bg);
  color: var(--accent-color);
}

.count {
  padding: 2px 8px;
  background: var(--bg-tertiary);
  border-radius: 10px;
  font-size: 12px;
}

.tab.active .count {
  background: var(--accent-color);
  color: white;
}

.albums-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(180px, 1fr));
  gap: 20px;
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

.empty {
  text-align: center;
  padding: 60px 20px;
  color: var(--text-tertiary);
}
</style>

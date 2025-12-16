<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useLibraryStore } from '../stores/library'
import { usePlayerStore } from '../stores/player'
import AlbumCard from '../components/AlbumCard.vue'
import SongList from '../components/SongList.vue'
import type { Song, Album } from '../api/subsonic'

const libraryStore = useLibraryStore()
const playerStore = usePlayerStore()

const recentAlbums = ref<Album[]>([])
const randomSongs = ref<Song[]>([])
const mostPlayedSongs = ref<Song[]>([])
const recentlyPlayedSongs = ref<Song[]>([])
const isLoading = ref(true)

onMounted(async () => {
  isLoading.value = true
  
  try {
    const [albums, random, mostPlayed, recentlyPlayed] = await Promise.all([
      libraryStore.fetchAlbums('newest', { size: 12 }).then(() => libraryStore.albums),
      libraryStore.fetchRandomSongs(20),
      libraryStore.fetchSongsList('mostPlayed', 10),
      libraryStore.fetchSongsList('recentlyPlayed', 10)
    ])
    
    recentAlbums.value = albums.slice(0, 12)
    randomSongs.value = random
    mostPlayedSongs.value = mostPlayed
    recentlyPlayedSongs.value = recentlyPlayed
  } catch (e) {
    console.error('Failed to load home data:', e)
  } finally {
    isLoading.value = false
  }
})

function playRandomSongs() {
  if (randomSongs.value.length > 0) {
    playerStore.setQueueAndPlay(randomSongs.value)
  }
}

function playMostPlayed() {
  if (mostPlayedSongs.value.length > 0) {
    playerStore.setQueueAndPlay(mostPlayedSongs.value)
  }
}

function playRecentlyPlayed() {
  if (recentlyPlayedSongs.value.length > 0) {
    playerStore.setQueueAndPlay(recentlyPlayedSongs.value)
  }
}

async function refreshRandom() {
  randomSongs.value = await libraryStore.fetchRandomSongs(20)
}
</script>

<template>
  <div class="home-view">
    <!-- 第一行：最多播放 + 最近播放 -->
    <div class="top-row">
      <section class="section half">
        <div class="section-header">
          <h2>最多播放</h2>
          <button class="action-btn primary" @click="playMostPlayed">▶️ 播放</button>
        </div>
        <SongList
          v-if="!isLoading && mostPlayedSongs.length > 0"
          :songs="mostPlayedSongs"
          :show-album="true"
        />
        <div class="loading" v-else-if="isLoading">加载中...</div>
        <div class="empty" v-else>暂无数据</div>
      </section>

      <section class="section half">
        <div class="section-header">
          <h2>最近播放</h2>
          <button class="action-btn primary" @click="playRecentlyPlayed">▶️ 播放</button>
        </div>
        <SongList
          v-if="!isLoading && recentlyPlayedSongs.length > 0"
          :songs="recentlyPlayedSongs"
          :show-album="true"
        />
        <div class="loading" v-else-if="isLoading">加载中...</div>
        <div class="empty" v-else>暂无数据</div>
      </section>
    </div>

    <!-- 第二行：随机播放 -->
    <section class="section">
      <div class="section-header">
        <h2>随机播放</h2>
        <div class="section-actions">
          <button class="action-btn" @click="refreshRandom">🔄 换一批</button>
          <button class="action-btn primary" @click="playRandomSongs">▶️ 播放全部</button>
        </div>
      </div>
      
      <SongList
        v-if="!isLoading && randomSongs.length > 0"
        :songs="randomSongs"
        :show-album="true"
      />
      <div class="loading" v-else-if="isLoading">加载中...</div>
      <div class="empty" v-else>暂无歌曲</div>
    </section>

    <!-- 第三行：最近添加专辑 -->
    <section class="section">
      <div class="section-header">
        <h2>最近添加</h2>
        <router-link to="/albums" class="see-all">查看全部 →</router-link>
      </div>
      
      <div class="albums-grid" v-if="!isLoading">
        <AlbumCard
          v-for="album in recentAlbums"
          :key="album.id"
          :album="album"
        />
      </div>
      <div class="loading" v-else>加载中...</div>
    </section>
  </div>
</template>

<style scoped>
.home-view {
  max-width: 1400px;
  margin: 0 auto;
}

.top-row {
  display: flex;
  gap: 24px;
  margin-bottom: 40px;
}

.section {
  margin-bottom: 40px;
}

.section.half {
  flex: 1;
  min-width: 0;
  margin-bottom: 0;
}

.section-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 20px;
}

.section-header h2 {
  font-size: 24px;
  font-weight: 700;
  color: var(--text-primary);
  margin: 0;
}

.see-all {
  font-size: 14px;
  color: var(--text-secondary);
  text-decoration: none;
  transition: color 0.2s;
}

.see-all:hover {
  color: var(--accent-color);
}

.section-actions {
  display: flex;
  gap: 12px;
}

.action-btn {
  padding: 8px 16px;
  background: var(--bg-tertiary);
  border: none;
  border-radius: 20px;
  font-size: 14px;
  color: var(--text-secondary);
  cursor: pointer;
  transition: all 0.2s;
}

.action-btn:hover {
  background: var(--bg-hover);
  color: var(--text-primary);
}

.action-btn.primary {
  background: var(--accent-color);
  color: white;
}

.action-btn.primary:hover {
  filter: brightness(1.1);
}

.albums-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(180px, 1fr));
  gap: 20px;
}

.loading,
.empty {
  text-align: center;
  padding: 40px;
  color: var(--text-tertiary);
}
</style>

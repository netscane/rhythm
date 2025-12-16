<script setup lang="ts">
import { computed } from 'vue'
import { useRouter } from 'vue-router'
import { usePlayerStore } from '../stores/player'
import { useLibraryStore } from '../stores/library'
import subsonicApi, { type Album } from '../api/subsonic'
import LazyImage from './LazyImage.vue'

const props = defineProps<{
  album: Album
}>()

const router = useRouter()
const playerStore = usePlayerStore()
const libraryStore = useLibraryStore()

const coverUrl = computed(() => {
  if (props.album.coverArt) {
    return subsonicApi.getCoverArtUrl(props.album.coverArt, 200)
  }
  return ''
})

const isStarred = computed(() => libraryStore.isStarred(props.album.id, 'album'))

function navigateToAlbum() {
  router.push(`/album/${props.album.id}`)
}

function navigateToArtist(e: MouseEvent) {
  e.stopPropagation()
  if (props.album.artistId) {
    router.push(`/artist/${props.album.artistId}`)
  }
}

async function playAlbum(e: MouseEvent) {
  e.stopPropagation()
  
  const albumData = await libraryStore.fetchAlbum(props.album.id)
  if (albumData?.song) {
    playerStore.setQueueAndPlay(albumData.song)
  }
}

async function toggleStar(e: MouseEvent) {
  e.stopPropagation()
  await libraryStore.toggleStar(props.album.id, 'album')
}
</script>

<template>
  <div class="album-card" @click="navigateToAlbum">
    <div class="album-cover">
      <LazyImage 
        v-if="coverUrl" 
        :src="coverUrl" 
        :alt="album.name" 
        placeholder="💿"
      />
      <span v-else class="placeholder">💿</span>
      <div class="cover-actions">
        <button class="star-btn" :class="{ starred: isStarred }" @click="toggleStar" :title="isStarred ? '取消收藏' : '收藏'">
          {{ isStarred ? '❤️' : '🤍' }}
        </button>
        <button class="play-btn" @click="playAlbum">▶️</button>
      </div>
    </div>
    <div class="album-info">
      <div class="album-name" :title="album.name">{{ album.name }}</div>
      <div class="album-artist" @click="navigateToArtist">{{ album.artist || '未知艺术家' }}</div>
      <div class="album-year" v-if="album.year">{{ album.year }}</div>
    </div>
  </div>
</template>

<style scoped>
.album-card {
  background: var(--bg-secondary);
  border-radius: 8px;
  padding: 12px;
  cursor: pointer;
  transition: all 0.2s;
}

.album-card:hover {
  background: var(--bg-hover);
  transform: translateY(-4px);
}

.album-cover {
  position: relative;
  width: 100%;
  aspect-ratio: 1;
  border-radius: 4px;
  overflow: hidden;
  background: var(--bg-tertiary);
  margin-bottom: 12px;
}

.album-cover img {
  width: 100%;
  height: 100%;
  object-fit: cover;
}

.album-cover .placeholder {
  position: absolute;
  inset: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 48px;
}

.cover-actions {
  position: absolute;
  right: 8px;
  bottom: 8px;
  display: flex;
  gap: 8px;
  opacity: 0;
  transform: translateY(8px);
  transition: all 0.2s;
}

.album-card:hover .cover-actions {
  opacity: 1;
  transform: translateY(0);
}

.star-btn,
.play-btn {
  width: 36px;
  height: 36px;
  border-radius: 50%;
  background: var(--bg-secondary);
  border: none;
  font-size: 14px;
  cursor: pointer;
  transition: all 0.2s;
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.3);
}

.play-btn {
  width: 40px;
  height: 40px;
  background: var(--accent-color);
  font-size: 16px;
}

.star-btn:hover,
.play-btn:hover {
  transform: scale(1.1);
}

.star-btn.starred {
  background: var(--accent-color);
}

.album-info {
  min-width: 0;
}

.album-name {
  font-size: 14px;
  font-weight: 600;
  color: var(--text-primary);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  margin-bottom: 4px;
}

.album-artist {
  font-size: 12px;
  color: var(--text-secondary);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.album-artist:hover {
  text-decoration: underline;
}

.album-year {
  font-size: 11px;
  color: var(--text-tertiary);
  margin-top: 2px;
}
</style>

<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue'
import { useRouter } from 'vue-router'
import { usePlayerStore } from '../stores/player'
import { useLibraryStore } from '../stores/library'
import subsonicApi, { type Song } from '../api/subsonic'

const props = withDefaults(defineProps<{
  songs: Song[]
  showAlbum?: boolean
  showTrackNumber?: boolean
}>(), {
  showAlbum: false,
  showTrackNumber: false
})

const router = useRouter()
const playerStore = usePlayerStore()
const libraryStore = useLibraryStore()

const showPlaylistMenu = ref<string | null>(null)
const menuPosition = ref({ x: 0, y: 0 })

function getCoverUrl(coverArt?: string) {
  if (coverArt) {
    return subsonicApi.getCoverArtUrl(coverArt, 50)
  }
  return ''
}

function formatDuration(seconds?: number): string {
  if (!seconds) return '0:00'
  const mins = Math.floor(seconds / 60)
  const secs = Math.floor(seconds % 60)
  return `${mins}:${secs.toString().padStart(2, '0')}`
}

function playSong(index: number) {
  playerStore.setQueueAndPlay(props.songs, index)
}

function navigateToAlbum(albumId?: string, e?: MouseEvent) {
  e?.stopPropagation()
  if (albumId) {
    router.push(`/album/${albumId}`)
  }
}

function navigateToArtist(artistId?: string, e?: MouseEvent) {
  e?.stopPropagation()
  if (artistId) {
    router.push(`/artist/${artistId}`)
  }
}

function isCurrentSong(song: Song): boolean {
  return playerStore.currentSong?.id === song.id
}

function isStarred(songId: string): boolean {
  return libraryStore.starred.song.some(s => s.id === songId)
}

async function toggleStar(song: Song, e: MouseEvent) {
  e.stopPropagation()
  
  if (isStarred(song.id)) {
    await libraryStore.unstar(song.id, 'song')
  } else {
    await libraryStore.star(song.id, 'song')
  }
  await libraryStore.fetchStarred()
}

function addToQueue(song: Song, e: MouseEvent) {
  e.stopPropagation()
  playerStore.addToQueue(song)
}

function openPlaylistMenu(songId: string, e: MouseEvent) {
  e.stopPropagation()
  
  // 确保播放列表已加载
  if (libraryStore.playlists.length === 0) {
    libraryStore.fetchPlaylists()
  }
  
  const rect = (e.target as HTMLElement).getBoundingClientRect()
  menuPosition.value = {
    x: rect.left,
    y: rect.bottom + 4
  }
  showPlaylistMenu.value = songId
}

async function addToPlaylist(playlistId: string, songId: string) {
  await libraryStore.addToPlaylist(playlistId, [songId])
  showPlaylistMenu.value = null
}

function closePlaylistMenu() {
  showPlaylistMenu.value = null
}

onMounted(() => {
  document.addEventListener('click', closePlaylistMenu)
})

onUnmounted(() => {
  document.removeEventListener('click', closePlaylistMenu)
})
</script>

<template>
  <div class="song-list">
    <div
      v-for="(song, index) in songs"
      :key="song.id"
      class="song-item"
      :class="{ playing: isCurrentSong(song) }"
      @click="playSong(index)"
    >
      <div class="song-index">
        <span class="index-number" v-if="!isCurrentSong(song)">
          {{ showTrackNumber ? (song.track || index + 1) : index + 1 }}
        </span>
        <span class="playing-icon" v-else>🎵</span>
      </div>

      <div class="song-cover" v-if="showAlbum">
        <img v-if="getCoverUrl(song.coverArt)" :src="getCoverUrl(song.coverArt)" />
        <span v-else>🎵</span>
      </div>

      <div class="song-info">
        <div class="song-title">{{ song.title }}</div>
        <div class="song-meta">
          <span class="artist" @click="navigateToArtist(song.artistId, $event)">
            {{ song.artist || '未知艺术家' }}
          </span>
          <template v-if="showAlbum && song.album">
            <span class="separator">•</span>
            <span class="album" @click="navigateToAlbum(song.albumId, $event)">
              {{ song.album }}
            </span>
          </template>
        </div>
      </div>

      <div class="song-actions">
        <button
          class="action-btn star-btn"
          :class="{ starred: isStarred(song.id) }"
          @click="toggleStar(song, $event)"
          title="收藏"
        >
          {{ isStarred(song.id) ? '⭐' : '☆' }}
        </button>
        <button
          class="action-btn"
          @click="addToQueue(song, $event)"
          title="添加到队列"
        >
          ➕
        </button>
        <button
          class="action-btn"
          @click="openPlaylistMenu(song.id, $event)"
          title="添加到播放列表"
        >
          📋
        </button>
        
        <!-- 播放列表选择菜单 -->
        <div 
          v-if="showPlaylistMenu === song.id" 
          class="playlist-menu"
          :style="{ left: menuPosition.x + 'px', top: menuPosition.y + 'px' }"
          @click.stop
        >
          <div class="playlist-menu-header">添加到播放列表</div>
          <div 
            v-if="libraryStore.playlists.length === 0" 
            class="playlist-menu-empty"
          >
            暂无播放列表
          </div>
          <div 
            v-for="playlist in libraryStore.playlists" 
            :key="playlist.id"
            class="playlist-menu-item"
            @click="addToPlaylist(playlist.id, song.id)"
          >
            {{ playlist.name }}
          </div>
        </div>
      </div>

      <div class="song-duration">
        {{ formatDuration(song.duration) }}
      </div>
    </div>
  </div>
</template>

<style scoped>
.song-list {
  display: flex;
  flex-direction: column;
}

.song-item {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 8px 12px;
  border-radius: 8px;
  cursor: pointer;
  transition: background 0.2s;
}

.song-item:hover {
  background: var(--bg-hover);
}

.song-item.playing {
  background: var(--accent-bg);
}

.song-item.playing .song-title {
  color: var(--accent-color);
}

.song-index {
  width: 32px;
  text-align: center;
  font-size: 14px;
  color: var(--text-tertiary);
  flex-shrink: 0;
}

.playing-icon {
  animation: pulse 1s ease-in-out infinite;
}

@keyframes pulse {
  0%, 100% { opacity: 1; }
  50% { opacity: 0.5; }
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
  flex-shrink: 0;
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

.song-meta {
  font-size: 12px;
  color: var(--text-tertiary);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  margin-top: 2px;
}

.artist:hover,
.album:hover {
  color: var(--text-secondary);
  text-decoration: underline;
}

.separator {
  margin: 0 4px;
  opacity: 0.5;
}

.song-actions {
  display: flex;
  gap: 4px;
  opacity: 0;
  transition: opacity 0.2s;
}

.song-item:hover .song-actions {
  opacity: 1;
}

.action-btn {
  background: none;
  border: none;
  font-size: 14px;
  padding: 4px 8px;
  cursor: pointer;
  opacity: 0.6;
  transition: opacity 0.2s;
}

.action-btn:hover {
  opacity: 1;
}

.star-btn.starred {
  opacity: 1;
}

.playlist-menu {
  position: fixed;
  min-width: 180px;
  max-width: 280px;
  max-height: 300px;
  overflow-y: auto;
  background: var(--bg-secondary);
  border: 1px solid var(--border-color);
  border-radius: 8px;
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.3);
  z-index: 1000;
}

.playlist-menu-header {
  padding: 10px 12px;
  font-size: 12px;
  font-weight: 600;
  color: var(--text-secondary);
  border-bottom: 1px solid var(--border-color);
}

.playlist-menu-empty {
  padding: 16px 12px;
  font-size: 13px;
  color: var(--text-tertiary);
  text-align: center;
}

.playlist-menu-item {
  padding: 10px 12px;
  font-size: 13px;
  color: var(--text-primary);
  cursor: pointer;
  transition: background 0.2s;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.playlist-menu-item:hover {
  background: var(--bg-hover);
}

.song-duration {
  font-size: 13px;
  color: var(--text-tertiary);
  width: 48px;
  text-align: right;
  flex-shrink: 0;
}
</style>

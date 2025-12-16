<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { useRouter } from 'vue-router'
import { usePlayerStore } from '../stores/player'
import { useLibraryStore } from '../stores/library'
import { useSettingsStore, STREAM_FORMATS, checkAudioFormatSupport, type StreamFormat } from '../stores/settings'
import subsonicApi from '../api/subsonic'

const router = useRouter()
const playerStore = usePlayerStore()
const libraryStore = useLibraryStore()
const settingsStore = useSettingsStore()

const showQueue = ref(false)
const showFormatMenu = ref(false)
const formatSupport = ref<Map<StreamFormat, boolean>>(new Map())

onMounted(() => {
  formatSupport.value = checkAudioFormatSupport()
})

const coverUrl = computed(() => {
  if (playerStore.currentSong?.coverArt) {
    return subsonicApi.getCoverArtUrl(playerStore.currentSong.coverArt, 100)
  }
  return ''
})

const formattedCurrentTime = computed(() => formatTime(playerStore.currentTime))
const formattedDuration = computed(() => formatTime(playerStore.duration))

function formatTime(seconds: number): string {
  if (!seconds || isNaN(seconds)) return '0:00'
  const mins = Math.floor(seconds / 60)
  const secs = Math.floor(seconds % 60)
  return `${mins}:${secs.toString().padStart(2, '0')}`
}

function handleProgressClick(e: MouseEvent) {
  const target = e.currentTarget as HTMLElement
  const rect = target.getBoundingClientRect()
  const percent = ((e.clientX - rect.left) / rect.width) * 100
  playerStore.seekPercent(percent)
}

function handleVolumeChange(e: Event) {
  const target = e.target as HTMLInputElement
  const newVolume = parseFloat(target.value)
  playerStore.setVolume(newVolume)
}

const repeatIcon = computed(() => {
  switch (playerStore.repeatMode) {
    case 'one': return '🔂'
    case 'all': return '🔁'
    default: return '➡️'
  }
})

const isStarred = computed(() => {
  if (!playerStore.currentSong) return false
  return libraryStore.starred.song.some(s => s.id === playerStore.currentSong?.id)
})

async function toggleStar() {
  if (!playerStore.currentSong) return
  
  if (isStarred.value) {
    await libraryStore.unstar(playerStore.currentSong.id, 'song')
  } else {
    await libraryStore.star(playerStore.currentSong.id, 'song')
  }
  await libraryStore.fetchStarred()
}

function navigateToAlbum() {
  if (playerStore.currentSong?.albumId) {
    router.push(`/album/${playerStore.currentSong.albumId}`)
  }
}

function toggleQueue() {
  showQueue.value = !showQueue.value
}

function toggleFormatMenu() {
  showFormatMenu.value = !showFormatMenu.value
}

function selectFormat(format: StreamFormat) {
  settingsStore.setCurrentFormat(format)
  showFormatMenu.value = false
  
  // 如果有当前歌曲，重新加载以应用新格式
  if (playerStore.currentSong) {
    playerStore.reloadCurrentSong()
  }
}

const currentFormatLabel = computed(() => {
  const format = STREAM_FORMATS.find(f => f.value === settingsStore.currentFormat)
  return format?.label || '原始格式'
})

function getSongCoverUrl(coverArt?: string) {
  if (coverArt) {
    return subsonicApi.getCoverArtUrl(coverArt, 40)
  }
  return ''
}
</script>

<template>
  <div class="player-bar">
    <!-- 当前歌曲信息 -->
    <div class="song-info">
      <div 
        class="song-info-clickable"
        :class="{ clickable: !!playerStore.currentSong?.albumId }"
        @click="navigateToAlbum"
      >
        <div 
          class="cover" 
          :class="{ rotating: playerStore.isPlaying }"
          v-if="coverUrl"
        >
          <img :src="coverUrl" alt="Cover" />
        </div>
        <div 
          class="cover placeholder" 
          :class="{ rotating: playerStore.isPlaying }"
          v-else
        >
          <span>🎵</span>
        </div>
        
        <div class="song-details" v-if="playerStore.currentSong">
          <div class="song-title">{{ playerStore.currentSong.title }}</div>
          <div class="song-artist">{{ playerStore.currentSong.artist }}</div>
        </div>
        <div class="song-details" v-else>
          <div class="song-title">未播放</div>
          <div class="song-artist">选择一首歌曲开始播放</div>
        </div>
      </div>

      <button 
        class="star-btn" 
        :class="{ starred: isStarred }"
        @click.stop="toggleStar"
        v-if="playerStore.currentSong"
      >
        {{ isStarred ? '⭐' : '☆' }}
      </button>
    </div>

    <!-- 播放控制 -->
    <div class="player-controls">
      <div class="control-buttons">
        <button 
          class="control-btn" 
          :class="{ active: playerStore.isShuffled }"
          @click.stop="playerStore.toggleShuffle"
          title="随机播放"
        >
          🔀
        </button>
        <button 
          class="control-btn" 
          :disabled="!playerStore.hasPrevious"
          @click.stop="playerStore.previous"
          title="上一首"
        >
          ⏮️
        </button>
        <button 
          class="control-btn play-btn" 
          @click.stop="playerStore.togglePlay"
          :disabled="!playerStore.currentSong"
          title="播放/暂停"
        >
          {{ playerStore.isPlaying ? '⏸️' : '▶️' }}
        </button>
        <button 
          class="control-btn" 
          :disabled="!playerStore.hasNext"
          @click.stop="playerStore.next"
          title="下一首"
        >
          ⏭️
        </button>
        <button 
          class="control-btn" 
          :class="{ active: playerStore.repeatMode !== 'off' }"
          @click.stop="playerStore.toggleRepeat"
          :title="playerStore.repeatMode === 'off' ? '关闭循环' : playerStore.repeatMode === 'all' ? '列表循环' : '单曲循环'"
        >
          {{ repeatIcon }}
        </button>
      </div>

      <div class="progress-container">
        <span class="time">{{ formattedCurrentTime }}</span>
        <div class="progress-bar" @click="handleProgressClick">
          <div class="progress-track">
            <div class="progress-fill" :style="{ width: `${playerStore.progress}%` }"></div>
          </div>
        </div>
        <span class="time">{{ formattedDuration }}</span>
      </div>
    </div>

    <!-- 音量控制 -->
    <div class="volume-controls">
      <!-- 格式切换 -->
      <div class="format-selector">
        <button 
          class="format-btn"
          @click.stop="toggleFormatMenu"
          title="切换播放格式"
        >
          {{ currentFormatLabel }}
        </button>
        <div v-if="showFormatMenu" class="format-menu" @click.stop>
          <div 
            v-for="format in STREAM_FORMATS" 
            :key="format.value"
            class="format-menu-item"
            :class="{ 
              active: settingsStore.currentFormat === format.value,
              disabled: !formatSupport.get(format.value)
            }"
            @click="formatSupport.get(format.value) && selectFormat(format.value)"
          >
            {{ format.label }}
            <span v-if="!formatSupport.get(format.value)" class="unsupported-badge">不支持</span>
            <span v-else-if="settingsStore.defaultFormat === format.value" class="default-badge">默认</span>
          </div>
        </div>
      </div>

      <button class="control-btn" @click.stop="playerStore.toggleMute">
        {{ playerStore.isMuted || playerStore.volume === 0 ? '🔇' : playerStore.volume < 0.5 ? '🔉' : '🔊' }}
      </button>
      <input
        type="range"
        class="volume-slider"
        min="0"
        max="1"
        step="0.01"
        :value="playerStore.isMuted ? 0 : playerStore.volume"
        @input="handleVolumeChange"
        @change="handleVolumeChange"
      />
      <button 
        class="control-btn queue-btn" 
        :class="{ active: showQueue }"
        @click.stop="toggleQueue"
        title="播放队列"
      >
        📋
      </button>
    </div>

    <!-- 播放队列面板 -->
    <div v-if="showQueue" class="queue-panel">
      <div class="queue-header">
        <h3>播放队列</h3>
        <span class="queue-count">{{ playerStore.queue.length }} 首歌曲</span>
        <button class="queue-close" @click="showQueue = false">✕</button>
      </div>
      <div class="queue-list">
        <div 
          v-for="(song, index) in playerStore.queue" 
          :key="song.id + '-' + index"
          class="queue-item"
          :class="{ active: index === playerStore.currentIndex }"
          @click="playerStore.playIndex(index)"
        >
          <img 
            v-if="getSongCoverUrl(song.coverArt)" 
            :src="getSongCoverUrl(song.coverArt)" 
            class="queue-item-cover"
          />
          <span v-else class="queue-item-cover placeholder">🎵</span>
          <div class="queue-item-info">
            <div class="queue-item-title">{{ song.title }}</div>
            <div class="queue-item-artist">{{ song.artist }}</div>
          </div>
          <button 
            class="queue-item-remove" 
            @click.stop="playerStore.removeFromQueue(index)"
            title="移除"
          >
            ✕
          </button>
        </div>
        <div v-if="playerStore.queue.length === 0" class="queue-empty">
          队列为空
        </div>
      </div>
      <div class="queue-footer" v-if="playerStore.queue.length > 0">
        <button class="queue-clear" @click="playerStore.clearQueue()">清空队列</button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.player-bar {
  position: fixed;
  bottom: 0;
  left: 0;
  right: 0;
  height: 90px;
  background: var(--bg-secondary);
  border-top: 1px solid var(--border-color);
  display: flex;
  align-items: center;
  padding: 0 16px;
  z-index: 200;
}

.song-info {
  display: flex;
  align-items: center;
  gap: 12px;
  width: 280px;
  min-width: 180px;
}

.song-info-clickable {
  display: flex;
  align-items: center;
  gap: 12px;
  flex: 1;
  min-width: 0;
}

.song-info-clickable.clickable {
  cursor: pointer;
}

.cover {
  width: 56px;
  height: 56px;
  border-radius: 50%;
  overflow: hidden;
  flex-shrink: 0;
  animation: rotate 8s linear infinite;
  animation-play-state: paused;
}

.cover.rotating {
  animation-play-state: running;
}

@keyframes rotate {
  from {
    transform: rotate(0deg);
  }
  to {
    transform: rotate(360deg);
  }
}

.cover img {
  width: 100%;
  height: 100%;
  object-fit: cover;
  display: block;
  transform: scale(1.14);
}

.cover.placeholder {
  background: var(--bg-tertiary);
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 24px;
  border-radius: 50%;
}

.song-details {
  flex: 1;
  min-width: 0;
}

.song-title {
  font-size: 14px;
  font-weight: 600;
  color: var(--text-primary);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.song-artist {
  font-size: 12px;
  color: var(--text-secondary);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.star-btn {
  background: none;
  border: none;
  font-size: 18px;
  cursor: pointer;
  opacity: 0.6;
  transition: opacity 0.2s;
}

.star-btn:hover,
.star-btn.starred {
  opacity: 1;
}

.player-controls {
  flex: 1;
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 8px;
  max-width: 600px;
  margin: 0 auto;
}

.control-buttons {
  display: flex;
  align-items: center;
  gap: 16px;
  position: relative;
  z-index: 10;
}

.control-btn {
  background: none;
  border: none;
  font-size: 18px;
  cursor: pointer;
  opacity: 0.7;
  transition: all 0.2s;
  padding: 4px;
}

.control-btn:hover:not(:disabled) {
  opacity: 1;
  transform: scale(1.1);
}

.control-btn:disabled {
  opacity: 0.3;
  cursor: not-allowed;
}

.control-btn.active {
  opacity: 1;
  color: var(--accent-color);
}

.control-btn.play-btn {
  font-size: 28px;
}

.progress-container {
  display: flex;
  align-items: center;
  gap: 8px;
  width: 100%;
}

.time {
  font-size: 11px;
  color: var(--text-tertiary);
  width: 40px;
  text-align: center;
}

.progress-bar {
  flex: 1;
  height: 20px;
  display: flex;
  align-items: center;
  cursor: pointer;
}

.progress-track {
  width: 100%;
  height: 4px;
  background: var(--bg-tertiary);
  border-radius: 2px;
  overflow: hidden;
  position: relative;
}

.progress-fill {
  height: 100%;
  background: var(--accent-color);
  border-radius: 2px;
  transition: width 0.1s linear;
}

.progress-bar:hover .progress-track {
  height: 6px;
}

.volume-controls {
  display: flex;
  align-items: center;
  gap: 8px;
  width: 280px;
  justify-content: flex-end;
}

.format-selector {
  position: relative;
}

.format-btn {
  background: var(--bg-tertiary);
  border: none;
  color: var(--text-secondary);
  padding: 4px 10px;
  border-radius: 4px;
  font-size: 11px;
  cursor: pointer;
  transition: all 0.2s;
  white-space: nowrap;
}

.format-btn:hover {
  background: var(--bg-hover);
  color: var(--text-primary);
}

.format-menu {
  position: absolute;
  bottom: 100%;
  left: 50%;
  transform: translateX(-50%);
  margin-bottom: 8px;
  background: var(--bg-secondary);
  border: 1px solid var(--border-color);
  border-radius: 8px;
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.3);
  min-width: 120px;
  overflow: hidden;
  z-index: 1000;
}

.format-menu-item {
  padding: 10px 12px;
  font-size: 13px;
  color: var(--text-primary);
  cursor: pointer;
  transition: background 0.2s;
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
}

.format-menu-item:hover {
  background: var(--bg-hover);
}

.format-menu-item.active {
  color: var(--accent-color);
}

.default-badge {
  font-size: 10px;
  color: var(--text-tertiary);
  background: var(--bg-tertiary);
  padding: 2px 6px;
  border-radius: 4px;
}

.unsupported-badge {
  font-size: 10px;
  color: #ef4444;
  background: rgba(239, 68, 68, 0.1);
  padding: 2px 6px;
  border-radius: 4px;
}

.format-menu-item.disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.volume-slider {
  width: 100px;
  height: 4px;
  -webkit-appearance: none;
  background: var(--bg-tertiary);
  border-radius: 2px;
  cursor: pointer;
}

.volume-slider::-webkit-slider-thumb {
  -webkit-appearance: none;
  width: 12px;
  height: 12px;
  background: var(--accent-color);
  border-radius: 50%;
  cursor: pointer;
}

.queue-btn {
  margin-left: 8px;
}

/* 播放队列面板 */
.queue-panel {
  position: absolute;
  bottom: 100%;
  right: 16px;
  width: 360px;
  max-height: 480px;
  background: var(--bg-secondary);
  border: 1px solid var(--border-color);
  border-radius: 12px 12px 0 0;
  box-shadow: 0 -4px 20px rgba(0, 0, 0, 0.3);
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

.queue-header {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 16px;
  border-bottom: 1px solid var(--border-color);
}

.queue-header h3 {
  font-size: 16px;
  font-weight: 600;
  color: var(--text-primary);
  margin: 0;
}

.queue-count {
  font-size: 12px;
  color: var(--text-tertiary);
  flex: 1;
}

.queue-close {
  background: none;
  border: none;
  color: var(--text-secondary);
  cursor: pointer;
  font-size: 16px;
  padding: 4px;
}

.queue-close:hover {
  color: var(--text-primary);
}

.queue-list {
  flex: 1;
  overflow-y: auto;
  padding: 8px 0;
}

.queue-item {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 8px 16px;
  cursor: pointer;
  transition: background 0.2s;
}

.queue-item:hover {
  background: var(--bg-hover);
}

.queue-item.active {
  background: var(--accent-bg);
}

.queue-item-cover {
  width: 40px;
  height: 40px;
  border-radius: 4px;
  object-fit: cover;
  flex-shrink: 0;
}

.queue-item-cover.placeholder {
  background: var(--bg-tertiary);
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 16px;
}

.queue-item-info {
  flex: 1;
  min-width: 0;
}

.queue-item-title {
  font-size: 13px;
  font-weight: 500;
  color: var(--text-primary);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.queue-item.active .queue-item-title {
  color: var(--accent-color);
}

.queue-item-artist {
  font-size: 11px;
  color: var(--text-secondary);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.queue-item-remove {
  background: none;
  border: none;
  color: var(--text-tertiary);
  cursor: pointer;
  font-size: 12px;
  padding: 4px;
  opacity: 0;
  transition: opacity 0.2s;
}

.queue-item:hover .queue-item-remove {
  opacity: 1;
}

.queue-item-remove:hover {
  color: #ef4444;
}

.queue-empty {
  text-align: center;
  padding: 40px 20px;
  color: var(--text-tertiary);
  font-size: 14px;
}

.queue-footer {
  padding: 12px 16px;
  border-top: 1px solid var(--border-color);
}

.queue-clear {
  background: none;
  border: 1px solid var(--border-color);
  color: var(--text-secondary);
  padding: 8px 16px;
  border-radius: 6px;
  font-size: 12px;
  cursor: pointer;
  transition: all 0.2s;
}

.queue-clear:hover {
  background: var(--bg-hover);
  color: var(--text-primary);
}
</style>

<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue'
import { useAuthStore } from '../stores/auth'
import { useSettingsStore, STREAM_FORMATS, type StreamFormat } from '../stores/settings'
import subsonicApi, { type MusicFolder } from '../api/subsonic'

const authStore = useAuthStore()
const settingsStore = useSettingsStore()

const musicFolders = ref<MusicFolder[]>([])
const isLoadingFolders = ref(false)
const isScanning = ref(false)
const scanCount = ref(0)
const scanError = ref('')
let pollInterval: number | null = null

function handleFormatChange(e: Event) {
  const target = e.target as HTMLSelectElement
  settingsStore.setDefaultFormat(target.value as StreamFormat)
}

async function loadMusicFolders() {
  isLoadingFolders.value = true
  try {
    musicFolders.value = await subsonicApi.getMusicFolders()
  } catch (e) {
    console.error('Failed to load music folders:', e)
  } finally {
    isLoadingFolders.value = false
  }
}

async function checkScanStatus() {
  try {
    const status = await subsonicApi.getScanStatus()
    isScanning.value = status.scanning
    scanCount.value = status.count || 0
    
    if (!status.scanning && pollInterval) {
      clearInterval(pollInterval)
      pollInterval = null
    }
  } catch (e) {
    console.error('Failed to get scan status:', e)
  }
}

async function startScan() {
  scanError.value = ''
  try {
    const status = await subsonicApi.startScan()
    isScanning.value = status.scanning
    scanCount.value = status.count || 0
    
    // 开始轮询扫描状态
    if (status.scanning && !pollInterval) {
      pollInterval = window.setInterval(checkScanStatus, 2000)
    }
  } catch (e) {
    scanError.value = e instanceof Error ? e.message : '扫描失败'
  }
}

onMounted(() => {
  loadMusicFolders()
  checkScanStatus()
})

onUnmounted(() => {
  if (pollInterval) {
    clearInterval(pollInterval)
  }
})
</script>

<template>
  <div class="settings-view">
    <header class="page-header">
      <h1>设置</h1>
    </header>

    <div class="settings-content">
      <!-- 服务器信息 -->
      <section class="settings-section">
        <h2>服务器信息</h2>
        <div class="info-grid">
          <div class="info-item">
            <span class="info-label">服务器地址</span>
            <span class="info-value">{{ authStore.server }}</span>
          </div>
          <div class="info-item">
            <span class="info-label">用户名</span>
            <span class="info-value">{{ authStore.username }}</span>
          </div>
        </div>
      </section>

      <!-- 播放设置 -->
      <section class="settings-section">
        <h2>播放设置</h2>
        <div class="info-grid">
          <div class="info-item">
            <div class="setting-info">
              <span class="info-label">默认播放格式</span>
              <span class="setting-description">选择音频流的默认转码格式，"原始格式"表示不转码</span>
            </div>
            <select 
              class="format-select"
              :value="settingsStore.defaultFormat"
              @change="handleFormatChange"
            >
              <option 
                v-for="format in STREAM_FORMATS" 
                :key="format.value" 
                :value="format.value"
              >
                {{ format.label }}
              </option>
            </select>
          </div>
        </div>
      </section>

      <!-- 媒体库扫描 -->
      <section class="settings-section">
        <h2>媒体库</h2>
        
        <!-- 音乐文件夹列表 -->
        <div class="folders-section">
          <h3>音乐文件夹</h3>
          <div v-if="isLoadingFolders" class="loading-text">加载中...</div>
          <div v-else-if="musicFolders.length === 0" class="empty-text">暂无音乐文件夹</div>
          <ul v-else class="folders-list">
            <li v-for="folder in musicFolders" :key="folder.id" class="folder-item">
              <span class="folder-icon">📁</span>
              <span class="folder-name">{{ folder.name }}</span>
              <span class="folder-id">ID: {{ folder.id }}</span>
            </li>
          </ul>
        </div>

        <div class="scan-section">
          <p class="scan-description">
            扫描媒体库以更新音乐文件索引。如果您添加了新的音乐文件，请点击扫描。
          </p>
          
          <div v-if="isScanning" class="scan-status">
            <span class="scan-indicator"></span>
            <span>正在扫描... 已扫描 {{ scanCount }} 个文件</span>
          </div>
          
          <div v-if="scanError" class="scan-error">
            {{ scanError }}
          </div>
          
          <button 
            class="scan-btn" 
            @click="startScan" 
            :disabled="isScanning"
          >
            {{ isScanning ? '扫描中...' : '开始扫描' }}
          </button>
        </div>
      </section>
    </div>
  </div>
</template>

<style scoped>
.settings-view {
  max-width: 800px;
  margin: 0 auto;
}

.page-header {
  margin-bottom: 32px;
}

.page-header h1 {
  font-size: 28px;
  font-weight: 700;
  color: var(--text-primary);
  margin: 0;
}

.settings-content {
  display: flex;
  flex-direction: column;
  gap: 32px;
}

.settings-section {
  background: var(--bg-secondary);
  border-radius: 12px;
  padding: 24px;
}

.settings-section h2 {
  font-size: 18px;
  font-weight: 600;
  color: var(--text-primary);
  margin: 0 0 16px;
}

.info-grid {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.info-item {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 12px 0;
  border-bottom: 1px solid var(--border-color);
}

.info-item:last-child {
  border-bottom: none;
}

.info-label {
  font-size: 14px;
  color: var(--text-secondary);
}

.info-value {
  font-size: 14px;
  color: var(--text-primary);
  font-weight: 500;
}

.scan-section {
  display: flex;
  flex-direction: column;
  gap: 16px;
}

.scan-description {
  font-size: 14px;
  color: var(--text-secondary);
  margin: 0;
  line-height: 1.5;
}

.scan-status {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 12px 16px;
  background: var(--accent-bg);
  border-radius: 8px;
  font-size: 14px;
  color: var(--accent-color);
}

.scan-indicator {
  width: 8px;
  height: 8px;
  background: var(--accent-color);
  border-radius: 50%;
  animation: pulse 1.5s infinite;
}

@keyframes pulse {
  0%, 100% { opacity: 1; }
  50% { opacity: 0.4; }
}

.scan-error {
  padding: 12px 16px;
  background: rgba(239, 68, 68, 0.1);
  border: 1px solid rgba(239, 68, 68, 0.3);
  border-radius: 8px;
  color: #ef4444;
  font-size: 14px;
}

.scan-btn {
  align-self: flex-start;
  padding: 12px 24px;
  background: var(--accent-color);
  border: none;
  border-radius: 8px;
  font-size: 14px;
  font-weight: 600;
  color: white;
  cursor: pointer;
  transition: all 0.2s;
}

.scan-btn:hover:not(:disabled) {
  filter: brightness(1.1);
}

.scan-btn:disabled {
  opacity: 0.6;
  cursor: not-allowed;
}

.folders-section {
  margin-bottom: 24px;
  padding-bottom: 24px;
  border-bottom: 1px solid var(--border-color);
}

.folders-section h3 {
  font-size: 14px;
  font-weight: 600;
  color: var(--text-secondary);
  margin: 0 0 12px;
}

.folders-list {
  list-style: none;
  padding: 0;
  margin: 0;
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.folder-item {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 12px 16px;
  background: var(--bg-tertiary);
  border-radius: 8px;
}

.folder-icon {
  font-size: 18px;
}

.folder-name {
  flex: 1;
  font-size: 14px;
  color: var(--text-primary);
  font-weight: 500;
}

.folder-id {
  font-size: 12px;
  color: var(--text-tertiary);
}

.loading-text,
.empty-text {
  font-size: 14px;
  color: var(--text-tertiary);
  padding: 12px 0;
}

.setting-info {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.setting-description {
  font-size: 12px;
  color: var(--text-tertiary);
}

.format-select {
  padding: 8px 12px;
  background: var(--bg-tertiary);
  border: 1px solid var(--border-color);
  border-radius: 6px;
  font-size: 14px;
  color: var(--text-primary);
  cursor: pointer;
  min-width: 120px;
}

.format-select:focus {
  outline: none;
  border-color: var(--accent-color);
}
</style>

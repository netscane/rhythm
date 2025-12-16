import { defineStore } from 'pinia'
import { ref } from 'vue'

const STORAGE_KEY = 'rhythm-desk-settings'

export type StreamFormat = 'auto' | 'raw' | 'mp3' | 'opus' | 'aac' | 'flac'

export const STREAM_FORMATS: { value: StreamFormat; label: string; mimeTypes?: string[] }[] = [
  { value: 'auto', label: '自动' },
  { value: 'raw', label: '原始格式' },
  { value: 'mp3', label: 'MP3', mimeTypes: ['audio/mpeg', 'audio/mp3'] },
  { value: 'opus', label: 'Opus', mimeTypes: ['audio/ogg; codecs=opus', 'audio/opus', 'audio/webm; codecs=opus'] },
  { value: 'aac', label: 'AAC', mimeTypes: ['audio/aac', 'audio/mp4; codecs=mp4a.40.2', 'audio/x-m4a'] },
  { value: 'flac', label: 'FLAC', mimeTypes: ['audio/flac', 'audio/x-flac'] },
]

// 检测浏览器支持的音频格式
export function checkAudioFormatSupport(): Map<StreamFormat, boolean> {
  const audio = document.createElement('audio')
  const support = new Map<StreamFormat, boolean>()
  
  for (const format of STREAM_FORMATS) {
    if (!format.mimeTypes) {
      // auto 和 raw 始终支持
      support.set(format.value, true)
    } else {
      // 检测任一 MIME 类型是否支持
      const isSupported = format.mimeTypes.some(mimeType => {
        const canPlay = audio.canPlayType(mimeType)
        return canPlay === 'probably' || canPlay === 'maybe'
      })
      support.set(format.value, isSupported)
    }
  }
  
  return support
}

export const useSettingsStore = defineStore('settings', () => {
  // 默认播放格式
  const defaultFormat = ref<StreamFormat>('auto')
  // 当前播放格式（可在播放栏临时切换）
  const currentFormat = ref<StreamFormat>('auto')
  // 最大比特率 (0 = 不限制)
  const maxBitRate = ref<number>(0)

  // 从本地存储加载
  function loadFromStorage() {
    try {
      const stored = localStorage.getItem(STORAGE_KEY)
      if (stored) {
        const data = JSON.parse(stored)
        defaultFormat.value = data.defaultFormat || 'auto'
        currentFormat.value = data.defaultFormat || 'auto'
        maxBitRate.value = data.maxBitRate || 0
      }
    } catch (e) {
      console.error('Failed to load settings from storage:', e)
    }
  }

  // 保存到本地存储
  function saveToStorage() {
    try {
      localStorage.setItem(STORAGE_KEY, JSON.stringify({
        defaultFormat: defaultFormat.value,
        maxBitRate: maxBitRate.value
      }))
    } catch (e) {
      console.error('Failed to save settings to storage:', e)
    }
  }

  // 设置默认格式
  function setDefaultFormat(format: StreamFormat) {
    defaultFormat.value = format
    currentFormat.value = format
    saveToStorage()
  }

  // 临时切换当前格式（不保存）
  function setCurrentFormat(format: StreamFormat) {
    currentFormat.value = format
  }

  // 重置为默认格式
  function resetToDefaultFormat() {
    currentFormat.value = defaultFormat.value
  }

  // 设置最大比特率
  function setMaxBitRate(bitRate: number) {
    maxBitRate.value = bitRate
    saveToStorage()
  }

  // 获取流媒体参数
  function getStreamOptions() {
    const options: { format?: string; maxBitRate?: number; estimateContentLength?: boolean } = {}
    
    // auto: 不传 format 参数，由服务器决定
    // 其他格式（包括 raw）: 传递 format 参数
    if (currentFormat.value !== 'auto') {
      options.format = currentFormat.value
    }
    
    if (maxBitRate.value > 0) {
      options.maxBitRate = maxBitRate.value
    }
    
    // 当需要转码时（非 auto 且非 raw），请求预估内容大小
    if (currentFormat.value !== 'auto' && currentFormat.value !== 'raw') {
      options.estimateContentLength = true
    }
    
    console.log('getStreamOptions:', { currentFormat: currentFormat.value, options })
    return options
  }

  // 初始化时加载
  loadFromStorage()

  return {
    defaultFormat,
    currentFormat,
    maxBitRate,
    setDefaultFormat,
    setCurrentFormat,
    resetToDefaultFormat,
    setMaxBitRate,
    getStreamOptions,
    loadFromStorage
  }
})

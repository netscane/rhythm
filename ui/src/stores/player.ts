import { defineStore } from 'pinia'
import { ref, computed, watch } from 'vue'
import subsonicApi, { type Song } from '../api/subsonic'
import { useSettingsStore } from './settings'

export type RepeatMode = 'off' | 'all' | 'one'

export const usePlayerStore = defineStore('player', () => {
  // 播放队列
  const queue = ref<Song[]>([])
  const currentIndex = ref(-1)
  
  // 播放状态
  const isPlaying = ref(false)
  const currentTime = ref(0)
  const duration = ref(0)
  const volume = ref(1)
  const isMuted = ref(false)
  const repeatMode = ref<RepeatMode>('off')
  const isShuffled = ref(false)
  
  // 原始队列（用于 shuffle）
  const originalQueue = ref<Song[]>([])
  
  // Audio 元素（响应式引用）
  const audioElement = ref<HTMLAudioElement | null>(null)

  // 计算属性
  const currentSong = computed(() => {
    if (currentIndex.value >= 0 && currentIndex.value < queue.value.length) {
      return queue.value[currentIndex.value]
    }
    return null
  })

  const hasNext = computed(() => {
    if (repeatMode.value === 'all' || repeatMode.value === 'one') {
      return queue.value.length > 0
    }
    return currentIndex.value < queue.value.length - 1
  })

  const hasPrevious = computed(() => {
    if (repeatMode.value === 'all' || repeatMode.value === 'one') {
      return queue.value.length > 0
    }
    return currentIndex.value > 0
  })

  const progress = computed(() => {
    if (duration.value === 0) return 0
    return (currentTime.value / duration.value) * 100
  })

  // 初始化 Audio
  function initAudio() {
    if (audioElement.value) return

    const audio = new Audio()
    audio.volume = volume.value
    audio.crossOrigin = 'anonymous' // 需要用于 Web Audio API

    audio.addEventListener('timeupdate', () => {
      currentTime.value = audio.currentTime
    })

    audio.addEventListener('durationchange', () => {
      // 如果 audio.duration 是有效值则使用，否则使用歌曲元数据中的 duration
      if (audio.duration && isFinite(audio.duration)) {
        duration.value = audio.duration
      } else if (currentSong.value?.duration) {
        duration.value = currentSong.value.duration
      }
    })

    audio.addEventListener('ended', () => {
      handleSongEnd()
    })

    audio.addEventListener('play', () => {
      isPlaying.value = true
    })

    audio.addEventListener('pause', () => {
      isPlaying.value = false
    })

    audio.addEventListener('error', (e) => {
      console.error('Audio error:', e)
      isPlaying.value = false
    })
    
    audioElement.value = audio
  }

  // 处理歌曲结束
  function handleSongEnd() {
    // 发送 scrobble
    if (currentSong.value) {
      subsonicApi.scrobble(currentSong.value.id).catch(console.error)
    }

    if (repeatMode.value === 'one') {
      // 单曲循环
      if (audioElement.value) {
        audioElement.value.currentTime = 0
        audioElement.value.play()
      }
    } else if (hasNext.value) {
      next()
    } else {
      isPlaying.value = false
    }
  }

  // 播放指定歌曲
  function playSong(song: Song) {
    initAudio()
    
    const audio = audioElement.value
    if (!audio) return

    // 先用歌曲元数据设置 duration（防止 FLAC 等格式无法获取时长）
    if (song.duration) {
      duration.value = song.duration
    }

    const settingsStore = useSettingsStore()
    const url = subsonicApi.getStreamUrl(song.id, settingsStore.getStreamOptions())
    audio.src = url
    audio.play().catch(console.error)

    // 发送 scrobble (开始播放)
    subsonicApi.scrobble(song.id, undefined, false).catch(console.error)
  }

  // 重新加载当前歌曲（用于格式切换）
  function reloadCurrentSong() {
    const audio = audioElement.value
    const song = currentSong.value
    if (!audio || !song) return

    const wasPlaying = isPlaying.value
    const savedTime = currentTime.value

    const settingsStore = useSettingsStore()
    const url = subsonicApi.getStreamUrl(song.id, settingsStore.getStreamOptions())
    
    audio.src = url
    
    // 等待音频可以播放后恢复位置和状态
    const onCanPlay = () => {
      audio.removeEventListener('canplay', onCanPlay)
      if (savedTime > 0) {
        audio.currentTime = savedTime
      }
      if (wasPlaying) {
        audio.play().catch(console.error)
      }
    }
    
    audio.addEventListener('canplay', onCanPlay)
    audio.load()
  }

  // 播放/暂停
  function togglePlay() {
    const audio = audioElement.value
    if (!audio) return

    if (isPlaying.value) {
      audio.pause()
    } else {
      audio.play().catch(console.error)
    }
  }

  // 播放
  function play() {
    const audio = audioElement.value
    if (!audio) return
    audio.play().catch(console.error)
  }

  // 暂停
  function pause() {
    const audio = audioElement.value
    if (!audio) return
    audio.pause()
  }

  // 下一首
  function next() {
    if (queue.value.length === 0) return

    let nextIndex = currentIndex.value + 1
    if (nextIndex >= queue.value.length) {
      if (repeatMode.value === 'all') {
        nextIndex = 0
      } else {
        return
      }
    }

    currentIndex.value = nextIndex
    playSong(queue.value[nextIndex])
  }

  // 上一首
  function previous() {
    if (queue.value.length === 0) return

    // 如果播放超过 3 秒，重新播放当前歌曲
    if (currentTime.value > 3) {
      seek(0)
      return
    }

    let prevIndex = currentIndex.value - 1
    if (prevIndex < 0) {
      if (repeatMode.value === 'all') {
        prevIndex = queue.value.length - 1
      } else {
        return
      }
    }

    currentIndex.value = prevIndex
    playSong(queue.value[prevIndex])
  }

  // 跳转到指定时间
  function seek(time: number) {
    const audio = audioElement.value
    if (!audio) return
    audio.currentTime = time
  }

  // 跳转到指定百分比
  function seekPercent(percent: number) {
    const audio = audioElement.value
    if (!audio || duration.value === 0) return
    audio.currentTime = (percent / 100) * duration.value
  }

  // 设置音量
  function setVolume(vol: number) {
    const newVolume = Math.max(0, Math.min(1, vol))
    volume.value = newVolume
    
    // 如果正在调整音量且处于静音状态，自动取消静音
    if (isMuted.value && newVolume > 0) {
      isMuted.value = false
    }
    
    const audio = audioElement.value
    if (audio) {
      audio.volume = newVolume
    }
    
    console.log('Volume set to:', newVolume, 'Audio volume:', audio?.volume)
  }

  // 切换静音
  function toggleMute() {
    const audio = audioElement.value
    isMuted.value = !isMuted.value
    
    if (audio) {
      if (isMuted.value) {
        audio.volume = 0
      } else {
        audio.volume = volume.value
      }
    }
    
    console.log('Mute toggled:', isMuted.value, 'Volume:', volume.value, 'Audio volume:', audio?.volume)
  }

  // 切换循环模式
  function toggleRepeat() {
    const modes: RepeatMode[] = ['off', 'all', 'one']
    const currentIdx = modes.indexOf(repeatMode.value)
    repeatMode.value = modes[(currentIdx + 1) % modes.length]
  }

  // 切换随机播放
  function toggleShuffle() {
    // 如果队列为空或只有一首歌，不需要 shuffle
    if (queue.value.length <= 1) {
      isShuffled.value = !isShuffled.value
      return
    }
    
    if (isShuffled.value) {
      // 恢复原始顺序
      const currentSongId = currentSong.value?.id
      queue.value = [...originalQueue.value]
      if (currentSongId) {
        currentIndex.value = queue.value.findIndex(s => s.id === currentSongId)
        if (currentIndex.value === -1) currentIndex.value = 0
      }
    } else {
      // 保存原始顺序并打乱
      originalQueue.value = [...queue.value]
      const currentSongItem = currentSong.value
      
      // Fisher-Yates shuffle
      const shuffled = [...queue.value]
      for (let i = shuffled.length - 1; i > 0; i--) {
        const j = Math.floor(Math.random() * (i + 1))
        ;[shuffled[i], shuffled[j]] = [shuffled[j], shuffled[i]]
      }
      
      // 把当前歌曲移到开头
      if (currentSongItem) {
        const idx = shuffled.findIndex(s => s.id === currentSongItem.id)
        if (idx > 0) {
          shuffled.splice(idx, 1)
          shuffled.unshift(currentSongItem)
        }
        currentIndex.value = 0
      }
      
      queue.value = shuffled
    }
    isShuffled.value = !isShuffled.value
    
    console.log('Shuffle toggled:', isShuffled.value, 'Queue length:', queue.value.length)
  }

  // 设置队列并播放
  function setQueueAndPlay(songs: Song[], startIndex = 0) {
    queue.value = [...songs]
    originalQueue.value = [...songs]
    currentIndex.value = startIndex
    isShuffled.value = false
    
    if (songs.length > 0 && startIndex < songs.length) {
      playSong(songs[startIndex])
    }
  }

  // 添加到队列
  function addToQueue(songs: Song | Song[]) {
    const songsArray = Array.isArray(songs) ? songs : [songs]
    queue.value.push(...songsArray)
    originalQueue.value.push(...songsArray)
  }

  // 从队列移除
  function removeFromQueue(index: number) {
    if (index < 0 || index >= queue.value.length) return

    queue.value.splice(index, 1)
    
    if (index < currentIndex.value) {
      currentIndex.value--
    } else if (index === currentIndex.value) {
      const audio = audioElement.value
      if (queue.value.length === 0) {
        currentIndex.value = -1
        if (audio) {
          audio.pause()
          audio.src = ''
        }
      } else if (currentIndex.value >= queue.value.length) {
        currentIndex.value = queue.value.length - 1
      }
      if (currentSong.value) {
        playSong(currentSong.value)
      }
    }
  }

  // 清空队列
  function clearQueue() {
    queue.value = []
    originalQueue.value = []
    currentIndex.value = -1
    const audio = audioElement.value
    if (audio) {
      audio.pause()
      audio.src = ''
    }
    isPlaying.value = false
  }

  // 播放队列中的指定索引
  function playIndex(index: number) {
    if (index < 0 || index >= queue.value.length) return
    currentIndex.value = index
    playSong(queue.value[index])
  }

  // 保存播放队列到服务器
  async function savePlayQueue() {
    if (queue.value.length === 0) return
    
    try {
      await subsonicApi.savePlayQueue(
        queue.value.map(s => s.id),
        currentSong.value?.id,
        Math.floor(currentTime.value * 1000)
      )
    } catch (e) {
      console.error('Failed to save play queue:', e)
    }
  }

  // 从服务器恢复播放队列
  async function restorePlayQueue() {
    try {
      const playQueue = await subsonicApi.getPlayQueue()
      if (playQueue && playQueue.entry.length > 0) {
        queue.value = playQueue.entry
        originalQueue.value = playQueue.entry
        
        if (playQueue.current) {
          currentIndex.value = queue.value.findIndex(s => s.id === playQueue.current)
        }
        
        if (currentSong.value) {
          initAudio()
          const audio = audioElement.value
          if (audio) {
            const url = subsonicApi.getStreamUrl(currentSong.value.id)
            audio.src = url
            
            if (playQueue.position) {
              audio.currentTime = playQueue.position / 1000
            }
          }
        }
      }
    } catch (e) {
      console.error('Failed to restore play queue:', e)
    }
  }

  // 监听队列变化，自动保存
  watch([queue, currentIndex, currentTime], () => {
    // 可以在这里添加防抖保存逻辑
  }, { deep: true })

  return {
    // 状态
    queue,
    currentIndex,
    isPlaying,
    currentTime,
    duration,
    volume,
    isMuted,
    repeatMode,
    isShuffled,
    audioElement,
    
    // 计算属性
    currentSong,
    hasNext,
    hasPrevious,
    progress,
    
    // 方法
    initAudio,
    playSong,
    reloadCurrentSong,
    togglePlay,
    play,
    pause,
    next,
    previous,
    seek,
    seekPercent,
    setVolume,
    toggleMute,
    toggleRepeat,
    toggleShuffle,
    setQueueAndPlay,
    addToQueue,
    removeFromQueue,
    clearQueue,
    playIndex,
    savePlayQueue,
    restorePlayQueue
  }
})

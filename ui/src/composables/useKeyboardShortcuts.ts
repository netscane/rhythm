import { onMounted, onUnmounted } from 'vue'
import { usePlayerStore } from '../stores/player'

export function useKeyboardShortcuts() {
  const playerStore = usePlayerStore()

  function handleKeydown(e: KeyboardEvent) {
    // 忽略输入框中的按键
    const target = e.target as HTMLElement
    if (
      target.tagName === 'INPUT' ||
      target.tagName === 'TEXTAREA' ||
      target.isContentEditable
    ) {
      return
    }

    switch (e.code) {
      case 'Space':
        // 空格：播放/暂停
        e.preventDefault()
        playerStore.togglePlay()
        break

      case 'ArrowLeft':
        if (e.metaKey || e.ctrlKey) {
          // Cmd/Ctrl + 左箭头：上一首
          e.preventDefault()
          playerStore.previous()
        } else {
          // 左箭头：后退 5 秒
          e.preventDefault()
          playerStore.seek(Math.max(0, playerStore.currentTime - 5))
        }
        break

      case 'ArrowRight':
        if (e.metaKey || e.ctrlKey) {
          // Cmd/Ctrl + 右箭头：下一首
          e.preventDefault()
          playerStore.next()
        } else {
          // 右箭头：前进 5 秒
          e.preventDefault()
          playerStore.seek(Math.min(playerStore.duration, playerStore.currentTime + 5))
        }
        break

      case 'ArrowUp':
        // 上箭头：增加音量
        e.preventDefault()
        playerStore.setVolume(playerStore.volume + 0.1)
        break

      case 'ArrowDown':
        // 下箭头：减少音量
        e.preventDefault()
        playerStore.setVolume(playerStore.volume - 0.1)
        break

      case 'KeyM':
        // M：静音/取消静音
        e.preventDefault()
        playerStore.toggleMute()
        break

      case 'KeyR':
        // R：切换循环模式
        e.preventDefault()
        playerStore.toggleRepeat()
        break

      case 'KeyS':
        // S：切换随机播放
        e.preventDefault()
        playerStore.toggleShuffle()
        break
    }
  }

  onMounted(() => {
    window.addEventListener('keydown', handleKeydown)
  })

  onUnmounted(() => {
    window.removeEventListener('keydown', handleKeydown)
  })
}

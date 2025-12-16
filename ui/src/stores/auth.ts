import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import subsonicApi, { type SubsonicConfig } from '../api/subsonic'

const STORAGE_KEY = 'rhythm-desk-auth'
const SERVERS_HISTORY_KEY = 'rhythm-desk-servers'

export interface ServerHistory {
  server: string
  username: string
  password: string
  lastUsed: number
}

export const useAuthStore = defineStore('auth', () => {
  const server = ref('')
  const username = ref('')
  const password = ref('')
  const isAuthenticated = ref(false)
  const isLoading = ref(false)
  const error = ref<string | null>(null)
  const rememberCredentials = ref(true)
  const serverHistory = ref<ServerHistory[]>([])

  const config = computed<SubsonicConfig | null>(() => {
    if (!server.value || !username.value || !password.value) {
      return null
    }
    return {
      server: server.value,
      username: username.value,
      password: password.value
    }
  })

  // 加载服务器历史记录
  function loadServerHistory() {
    try {
      const stored = localStorage.getItem(SERVERS_HISTORY_KEY)
      if (stored) {
        serverHistory.value = JSON.parse(stored)
        // 按最后使用时间排序
        serverHistory.value.sort((a, b) => b.lastUsed - a.lastUsed)
      }
    } catch (e) {
      console.error('Failed to load server history:', e)
      serverHistory.value = []
    }
  }

  // 保存服务器到历史记录
  function saveServerToHistory(serverUrl: string, user: string, pass: string) {
    try {
      // 查找是否已存在
      const existingIndex = serverHistory.value.findIndex(
        s => s.server === serverUrl && s.username === user
      )

      const entry: ServerHistory = {
        server: serverUrl,
        username: user,
        password: rememberCredentials.value ? pass : '',
        lastUsed: Date.now()
      }

      if (existingIndex >= 0) {
        // 更新现有记录
        serverHistory.value[existingIndex] = entry
      } else {
        // 添加新记录，最多保存 10 个
        serverHistory.value.unshift(entry)
        if (serverHistory.value.length > 10) {
          serverHistory.value = serverHistory.value.slice(0, 10)
        }
      }

      // 按最后使用时间排序
      serverHistory.value.sort((a, b) => b.lastUsed - a.lastUsed)

      localStorage.setItem(SERVERS_HISTORY_KEY, JSON.stringify(serverHistory.value))
    } catch (e) {
      console.error('Failed to save server history:', e)
    }
  }

  // 删除历史服务器记录
  function removeServerFromHistory(serverUrl: string, user: string) {
    serverHistory.value = serverHistory.value.filter(
      s => !(s.server === serverUrl && s.username === user)
    )
    localStorage.setItem(SERVERS_HISTORY_KEY, JSON.stringify(serverHistory.value))
  }

  // 从本地存储加载配置
  function loadFromStorage() {
    try {
      loadServerHistory()

      const stored = localStorage.getItem(STORAGE_KEY)
      if (stored) {
        const data = JSON.parse(stored)
        server.value = data.server || ''
        username.value = data.username || ''
        password.value = data.password || ''
        rememberCredentials.value = data.rememberCredentials !== false

        if (config.value) {
          subsonicApi.configure(config.value)
          isAuthenticated.value = true
        }
      }
    } catch (e) {
      console.error('Failed to load auth from storage:', e)
    }
  }

  // 保存到本地存储
  function saveToStorage() {
    try {
      localStorage.setItem(STORAGE_KEY, JSON.stringify({
        server: server.value,
        username: username.value,
        password: rememberCredentials.value ? password.value : '',
        rememberCredentials: rememberCredentials.value
      }))
    } catch (e) {
      console.error('Failed to save auth to storage:', e)
    }
  }

  // 登录
  async function login(serverUrl: string, user: string, pass: string): Promise<boolean> {
    isLoading.value = true
    error.value = null

    try {
      // 规范化服务器 URL
      let normalizedUrl = serverUrl.trim()
      if (!normalizedUrl.startsWith('http://') && !normalizedUrl.startsWith('https://')) {
        normalizedUrl = 'http://' + normalizedUrl
      }
      if (normalizedUrl.endsWith('/')) {
        normalizedUrl = normalizedUrl.slice(0, -1)
      }

      const testConfig: SubsonicConfig = {
        server: normalizedUrl,
        username: user,
        password: pass
      }

      subsonicApi.configure(testConfig)
      const success = await subsonicApi.ping()

      if (success) {
        server.value = normalizedUrl
        username.value = user
        password.value = pass
        isAuthenticated.value = true
        saveToStorage()
        saveServerToHistory(normalizedUrl, user, pass)
        return true
      } else {
        error.value = '连接失败，请检查服务器地址和凭据'
        return false
      }
    } catch (e) {
      error.value = e instanceof Error ? e.message : '连接失败'
      return false
    } finally {
      isLoading.value = false
    }
  }

  // 登出
  function logout() {
    server.value = ''
    username.value = ''
    password.value = ''
    isAuthenticated.value = false
    localStorage.removeItem(STORAGE_KEY)
  }

  // 初始化时加载
  loadFromStorage()

  return {
    server,
    username,
    password,
    isAuthenticated,
    isLoading,
    error,
    config,
    rememberCredentials,
    serverHistory,
    login,
    logout,
    loadFromStorage,
    loadServerHistory,
    removeServerFromHistory
  }
})

<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { useRouter } from 'vue-router'
import { useAuthStore, type ServerHistory } from '../stores/auth'

const router = useRouter()
const authStore = useAuthStore()

const server = ref('')
const username = ref('')
const password = ref('')
const isLoading = ref(false)
const error = ref('')
const rememberCredentials = ref(true)
const showServerDropdown = ref(false)

const serverHistory = computed(() => authStore.serverHistory)

onMounted(() => {
  authStore.loadServerHistory()
  // 如果有历史记录，自动填充最近使用的
  if (serverHistory.value.length > 0) {
    const lastUsed = serverHistory.value[0]
    server.value = lastUsed.server
    username.value = lastUsed.username
    password.value = lastUsed.password
  }
})

function selectServer(item: ServerHistory) {
  server.value = item.server
  username.value = item.username
  password.value = item.password
  showServerDropdown.value = false
}

function removeServer(item: ServerHistory, event: Event) {
  event.stopPropagation()
  authStore.removeServerFromHistory(item.server, item.username)
}

function handleServerFocus() {
  if (serverHistory.value.length > 0) {
    showServerDropdown.value = true
  }
}

function handleServerBlur() {
  // 延迟关闭，让点击事件有时间触发
  setTimeout(() => {
    showServerDropdown.value = false
  }, 200)
}

async function handleLogin() {
  if (!server.value || !username.value || !password.value) {
    error.value = '请填写所有字段'
    return
  }

  isLoading.value = true
  error.value = ''
  authStore.rememberCredentials = rememberCredentials.value

  const success = await authStore.login(server.value, username.value, password.value)
  
  isLoading.value = false

  if (success) {
    router.push('/')
  } else {
    error.value = authStore.error || '登录失败'
  }
}
</script>

<template>
  <div class="login-page">
    <div class="login-card">
      <div class="login-header">
        <span class="logo">🎵</span>
        <h1>Rhythm Desk</h1>
        <p>连接到您的 Subsonic 服务器</p>
      </div>

      <form @submit.prevent="handleLogin" class="login-form">
        <div class="form-group">
          <label for="server">服务器地址</label>
          <div class="server-input-wrapper">
            <input
              id="server"
              v-model="server"
              type="text"
              placeholder="http://your-server.com:4533"
              :disabled="isLoading"
              autocomplete="off"
              @focus="handleServerFocus"
              @blur="handleServerBlur"
            />
            <div v-if="showServerDropdown && serverHistory.length > 0" class="server-dropdown">
              <div
                v-for="item in serverHistory"
                :key="item.server + item.username"
                class="server-item"
                @click="selectServer(item)"
              >
                <div class="server-info">
                  <span class="server-url">{{ item.server }}</span>
                  <span class="server-user">{{ item.username }}</span>
                </div>
                <button
                  type="button"
                  class="remove-btn"
                  @click="removeServer(item, $event)"
                  title="删除此记录"
                >
                  ×
                </button>
              </div>
            </div>
          </div>
        </div>

        <div class="form-group">
          <label for="username">用户名</label>
          <input
            id="username"
            v-model="username"
            type="text"
            placeholder="输入用户名"
            :disabled="isLoading"
          />
        </div>

        <div class="form-group">
          <label for="password">密码</label>
          <input
            id="password"
            v-model="password"
            type="password"
            placeholder="输入密码"
            :disabled="isLoading"
          />
        </div>

        <div class="form-group checkbox-group">
          <label class="checkbox-label">
            <input
              type="checkbox"
              v-model="rememberCredentials"
              :disabled="isLoading"
            />
            <span>记住账号和密码</span>
          </label>
        </div>

        <div v-if="error" class="error-message">
          {{ error }}
        </div>

        <button type="submit" class="login-btn" :disabled="isLoading">
          {{ isLoading ? '连接中...' : '连接' }}
        </button>
      </form>

      <div class="login-footer">
        <p>支持 Subsonic / Navidrome / Airsonic 等服务器</p>
      </div>
    </div>
  </div>
</template>

<style scoped>
.login-page {
  min-height: 100vh;
  display: flex;
  align-items: center;
  justify-content: center;
  background: linear-gradient(135deg, var(--bg-primary) 0%, var(--bg-secondary) 100%);
  padding: 20px;
}

.login-card {
  background: var(--bg-secondary);
  border-radius: 16px;
  padding: 40px;
  width: 100%;
  max-width: 400px;
  box-shadow: 0 8px 32px rgba(0, 0, 0, 0.3);
}

.login-header {
  text-align: center;
  margin-bottom: 32px;
}

.logo {
  font-size: 48px;
  display: block;
  margin-bottom: 16px;
}

.login-header h1 {
  font-size: 24px;
  font-weight: 700;
  color: var(--text-primary);
  margin: 0 0 8px;
}

.login-header p {
  font-size: 14px;
  color: var(--text-secondary);
  margin: 0;
}

.login-form {
  display: flex;
  flex-direction: column;
  gap: 20px;
}

.form-group {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.form-group label {
  font-size: 14px;
  font-weight: 500;
  color: var(--text-secondary);
}

.form-group input[type="text"],
.form-group input[type="password"] {
  width: 100%;
  padding: 12px 16px;
  background: var(--bg-tertiary);
  border: 1px solid var(--border-color);
  border-radius: 8px;
  font-size: 14px;
  color: var(--text-primary);
  transition: border-color 0.2s;
  box-sizing: border-box;
}

.form-group input:focus {
  outline: none;
  border-color: var(--accent-color);
}

.form-group input::placeholder {
  color: var(--text-tertiary);
}

/* Server dropdown */
.server-input-wrapper {
  position: relative;
}

.server-dropdown {
  position: absolute;
  top: 100%;
  left: 0;
  right: 0;
  background: var(--bg-tertiary);
  border: 1px solid var(--border-color);
  border-radius: 8px;
  margin-top: 4px;
  max-height: 200px;
  overflow-y: auto;
  z-index: 100;
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.2);
}

.server-item {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 10px 12px;
  cursor: pointer;
  transition: background 0.2s;
}

.server-item:hover {
  background: var(--bg-hover);
}

.server-item:not(:last-child) {
  border-bottom: 1px solid var(--border-color);
}

.server-info {
  display: flex;
  flex-direction: column;
  gap: 2px;
  overflow: hidden;
}

.server-url {
  font-size: 13px;
  color: var(--text-primary);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.server-user {
  font-size: 11px;
  color: var(--text-secondary);
}

.remove-btn {
  background: none;
  border: none;
  color: var(--text-tertiary);
  font-size: 18px;
  cursor: pointer;
  padding: 4px 8px;
  border-radius: 4px;
  transition: all 0.2s;
  flex-shrink: 0;
}

.remove-btn:hover {
  color: #ef4444;
  background: rgba(239, 68, 68, 0.1);
}

/* Checkbox */
.checkbox-group {
  flex-direction: row;
}

.checkbox-label {
  display: flex;
  align-items: center;
  gap: 8px;
  cursor: pointer;
  font-size: 14px;
  color: var(--text-secondary);
}

.checkbox-label input[type="checkbox"] {
  width: 16px;
  height: 16px;
  accent-color: var(--accent-color);
  cursor: pointer;
}

.error-message {
  padding: 12px;
  background: rgba(239, 68, 68, 0.1);
  border: 1px solid rgba(239, 68, 68, 0.3);
  border-radius: 8px;
  color: #ef4444;
  font-size: 14px;
  text-align: center;
}

.login-btn {
  padding: 14px;
  background: var(--accent-color);
  border: none;
  border-radius: 8px;
  font-size: 16px;
  font-weight: 600;
  color: white;
  cursor: pointer;
  transition: all 0.2s;
}

.login-btn:hover:not(:disabled) {
  filter: brightness(1.1);
}

.login-btn:disabled {
  opacity: 0.6;
  cursor: not-allowed;
}

.login-footer {
  margin-top: 24px;
  text-align: center;
}

.login-footer p {
  font-size: 12px;
  color: var(--text-tertiary);
  margin: 0;
}
</style>

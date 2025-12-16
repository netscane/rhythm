<script setup lang="ts">
import { ref } from 'vue'
import { useRouter } from 'vue-router'

const router = useRouter()
const query = ref('')

function handleSearch() {
  if (query.value.trim()) {
    router.push({ name: 'search', query: { q: query.value.trim() } })
  }
}
</script>

<template>
  <div class="search-bar">
    <span class="search-icon">🔍</span>
    <input
      v-model="query"
      type="text"
      placeholder="搜索艺术家、专辑、歌曲..."
      @keyup.enter="handleSearch"
    />
  </div>
</template>

<style scoped>
.search-bar {
  display: flex;
  align-items: center;
  gap: 8px;
  background: var(--bg-tertiary);
  border-radius: 20px;
  padding: 8px 16px;
  width: 100%;
  max-width: 400px;
}

.search-icon {
  font-size: 16px;
  opacity: 0.6;
}

input {
  flex: 1;
  background: none;
  border: none;
  outline: none;
  font-size: 14px;
  color: var(--text-primary);
}

input::placeholder {
  color: var(--text-tertiary);
}
</style>

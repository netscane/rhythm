<script setup lang="ts">
import { ref, onMounted, onUnmounted, watch } from 'vue'
import { imageLimiter } from '../utils/concurrency'

const props = withDefaults(defineProps<{
  src: string
  alt?: string
  placeholder?: string
}>(), {
  alt: '',
  placeholder: ''
})

const imgRef = ref<HTMLElement>()
const isLoaded = ref(false)
const isError = ref(false)
const isInView = ref(false)
const actualSrc = ref('')

let observer: IntersectionObserver | null = null

async function loadImage() {
  if (!props.src || isLoaded.value) return

  try {
    await imageLimiter.run(async () => {
      await new Promise<void>((resolve, reject) => {
        const img = new Image()
        img.onload = () => resolve()
        img.onerror = () => reject()
        img.src = props.src
      })
    })
    
    actualSrc.value = props.src
    isLoaded.value = true
  } catch {
    isError.value = true
  }
}

onMounted(() => {
  if (!imgRef.value) return

  observer = new IntersectionObserver(
    (entries) => {
      entries.forEach(entry => {
        if (entry.isIntersecting) {
          isInView.value = true
          loadImage()
          // 加载后停止观察
          observer?.unobserve(entry.target)
        }
      })
    },
    {
      rootMargin: '100px', // 提前 100px 开始加载
      threshold: 0
    }
  )

  observer.observe(imgRef.value)
})

onUnmounted(() => {
  observer?.disconnect()
})

// 当 src 变化时重新加载
watch(() => props.src, () => {
  if (isInView.value) {
    isLoaded.value = false
    isError.value = false
    loadImage()
  }
})
</script>

<template>
  <div ref="imgRef" class="lazy-image" :class="{ loaded: isLoaded, error: isError }">
    <img 
      v-if="isLoaded && actualSrc" 
      :src="actualSrc" 
      :alt="alt"
      class="image"
    />
    <div v-else-if="isError" class="placeholder error-placeholder">
      <slot name="error">
        <span>{{ placeholder || '🖼️' }}</span>
      </slot>
    </div>
    <div v-else class="placeholder loading-placeholder">
      <slot name="loading">
        <span>{{ placeholder || '🎵' }}</span>
      </slot>
    </div>
  </div>
</template>

<style scoped>
.lazy-image {
  width: 100%;
  height: 100%;
  position: relative;
  overflow: hidden;
  background: var(--bg-tertiary);
}

.image {
  width: 100%;
  height: 100%;
  object-fit: cover;
  opacity: 0;
  transition: opacity 0.3s ease;
}

.lazy-image.loaded .image {
  opacity: 1;
}

.placeholder {
  position: absolute;
  inset: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 24px;
  color: var(--text-tertiary);
}

.loading-placeholder {
  animation: pulse 1.5s ease-in-out infinite;
}

@keyframes pulse {
  0%, 100% { opacity: 0.5; }
  50% { opacity: 1; }
}
</style>

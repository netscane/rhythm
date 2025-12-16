/**
 * 并发限制器
 * 限制同时进行的异步操作数量
 */

export class ConcurrencyLimiter {
  private running = 0
  private queue: Array<() => void> = []

  constructor(private maxConcurrent: number = 6) { }

  async run<T>(fn: () => Promise<T>): Promise<T> {
    // 如果当前运行数达到上限，等待
    if (this.running >= this.maxConcurrent) {
      await new Promise<void>(resolve => this.queue.push(resolve))
    }

    this.running++

    try {
      return await fn()
    } finally {
      this.running--
      // 释放队列中的下一个任务
      const next = this.queue.shift()
      if (next) next()
    }
  }
}

// 图片加载专用限制器（限制 12 个并发）
export const imageLimiter = new ConcurrencyLimiter(12)

// API 请求限制器（限制 6 个并发）
export const apiLimiter = new ConcurrencyLimiter(6)

/**
 * 带并发限制的图片 URL 生成
 * 配合 IntersectionObserver 实现懒加载
 */
export function useLazyImage() {
  const loadedImages = new Map<string, string>()
  const loadingImages = new Set<string>()

  async function loadImage(url: string): Promise<string> {
    if (loadedImages.has(url)) {
      return loadedImages.get(url)!
    }

    if (loadingImages.has(url)) {
      // 等待已在加载的图片
      return new Promise(resolve => {
        const check = () => {
          if (loadedImages.has(url)) {
            resolve(loadedImages.get(url)!)
          } else if (loadingImages.has(url)) {
            setTimeout(check, 50)
          } else {
            resolve('')
          }
        }
        check()
      })
    }

    loadingImages.add(url)

    try {
      await imageLimiter.run(async () => {
        // 预加载图片
        await new Promise<void>((resolve, reject) => {
          const img = new Image()
          img.onload = () => resolve()
          img.onerror = () => reject()
          img.src = url
        })
      })

      loadedImages.set(url, url)
      return url
    } catch {
      return ''
    } finally {
      loadingImages.delete(url)
    }
  }

  return { loadImage, loadedImages }
}

/**
 * 批量执行带并发限制
 */
export async function runWithConcurrency<T, R>(
  items: T[],
  fn: (item: T) => Promise<R>,
  maxConcurrent = 6
): Promise<R[]> {
  const limiter = new ConcurrencyLimiter(maxConcurrent)
  return Promise.all(items.map(item => limiter.run(() => fn(item))))
}

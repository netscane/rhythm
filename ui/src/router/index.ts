import { createRouter, createWebHistory } from 'vue-router'
import { useAuthStore } from '../stores/auth'

const router = createRouter({
  history: createWebHistory(import.meta.env.BASE_URL),
  routes: [
    {
      path: '/login',
      name: 'login',
      component: () => import('../views/LoginView.vue'),
      meta: { requiresAuth: false }
    },
    {
      path: '/',
      component: () => import('../layouts/MainLayout.vue'),
      meta: { requiresAuth: true },
      children: [
        {
          path: '',
          name: 'home',
          component: () => import('../views/HomeView.vue')
        },
        {
          path: 'artists',
          name: 'artists',
          component: () => import('../views/ArtistsView.vue')
        },
        {
          path: 'artist/:id',
          name: 'artist',
          component: () => import('../views/ArtistDetailView.vue')
        },
        {
          path: 'albums',
          name: 'albums',
          component: () => import('../views/AlbumsView.vue')
        },
        {
          path: 'album/:id',
          name: 'album',
          component: () => import('../views/AlbumDetailView.vue')
        },
        {
          path: 'playlists',
          name: 'playlists',
          component: () => import('../views/PlaylistsView.vue')
        },
        {
          path: 'playlist/:id',
          name: 'playlist',
          component: () => import('../views/PlaylistDetailView.vue')
        },
        {
          path: 'genres',
          name: 'genres',
          component: () => import('../views/GenresView.vue')
        },
        {
          path: 'genre/:name',
          name: 'genre',
          component: () => import('../views/GenreDetailView.vue')
        },
        {
          path: 'starred',
          name: 'starred',
          component: () => import('../views/StarredView.vue')
        },
        {
          path: 'search',
          name: 'search',
          component: () => import('../views/SearchView.vue')
        },
        {
          path: 'settings',
          name: 'settings',
          component: () => import('../views/SettingsView.vue')
        }
      ]
    }
  ]
})

router.beforeEach((to, _from, next) => {
  const authStore = useAuthStore()
  
  if (to.meta.requiresAuth !== false && !authStore.isAuthenticated) {
    next({ name: 'login' })
  } else {
    next()
  }
})

export default router

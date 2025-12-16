import { defineStore } from 'pinia'
import { ref } from 'vue'
import subsonicApi, {
  type Artist,
  type ArtistWithAlbums,
  type Album,
  type AlbumWithSongs,
  type Song,
  type Playlist,
  type PlaylistWithSongs,
  type Genre,
  type Index,
  type SearchResult3
} from '../api/subsonic'

export const useLibraryStore = defineStore('library', () => {
  // 状态
  const artistIndexes = ref<Index[]>([])
  const albums = ref<Album[]>([])
  const genres = ref<Genre[]>([])
  const playlists = ref<Playlist[]>([])
  const starred = ref<{ artist: Artist[]; album: Album[]; song: Song[] }>({
    artist: [],
    album: [],
    song: []
  })
  
  // 收藏 ID 集合，用于快速查询
  const starredIds = ref<{ artists: Set<string>; albums: Set<string>; songs: Set<string> }>({
    artists: new Set(),
    albums: new Set(),
    songs: new Set()
  })

  // 加载状态
  const isLoadingArtists = ref(false)
  const isLoadingAlbums = ref(false)
  const isLoadingGenres = ref(false)
  const isLoadingPlaylists = ref(false)

  // 缓存
  const artistCache = ref<Map<string, ArtistWithAlbums>>(new Map())
  const albumCache = ref<Map<string, AlbumWithSongs>>(new Map())
  const playlistCache = ref<Map<string, PlaylistWithSongs>>(new Map())

  // ==================== 艺术家 ====================

  async function fetchArtists() {
    if (isLoadingArtists.value) return
    isLoadingArtists.value = true
    
    try {
      artistIndexes.value = await subsonicApi.getArtists()
    } catch (e) {
      console.error('Failed to fetch artists:', e)
    } finally {
      isLoadingArtists.value = false
    }
  }

  async function fetchArtist(id: string): Promise<ArtistWithAlbums | null> {
    if (artistCache.value.has(id)) {
      return artistCache.value.get(id)!
    }

    try {
      const artist = await subsonicApi.getArtist(id)
      artistCache.value.set(id, artist)
      return artist
    } catch (e) {
      console.error('Failed to fetch artist:', e)
      return null
    }
  }

  // ==================== 专辑 ====================

  async function fetchAlbums(
    type: 'random' | 'newest' | 'highest' | 'frequent' | 'recent' | 'alphabeticalByName' | 'alphabeticalByArtist' | 'starred' = 'newest',
    options: { size?: number; offset?: number } = {}
  ) {
    if (isLoadingAlbums.value) return
    isLoadingAlbums.value = true

    try {
      const newAlbums = await subsonicApi.getAlbumList2(type, {
        size: options.size || 50,
        offset: options.offset || 0
      })
      
      if (options.offset === 0) {
        albums.value = newAlbums
      } else {
        albums.value.push(...newAlbums)
      }
    } catch (e) {
      console.error('Failed to fetch albums:', e)
    } finally {
      isLoadingAlbums.value = false
    }
  }

  async function fetchAlbum(id: string): Promise<AlbumWithSongs | null> {
    if (albumCache.value.has(id)) {
      return albumCache.value.get(id)!
    }

    try {
      const album = await subsonicApi.getAlbum(id)
      albumCache.value.set(id, album)
      return album
    } catch (e) {
      console.error('Failed to fetch album:', e)
      return null
    }
  }

  // ==================== 流派 ====================

  async function fetchGenres() {
    if (isLoadingGenres.value) return
    isLoadingGenres.value = true

    try {
      genres.value = await subsonicApi.getGenres()
    } catch (e) {
      console.error('Failed to fetch genres:', e)
    } finally {
      isLoadingGenres.value = false
    }
  }

  async function fetchSongsByGenre(genre: string, count = 50, offset = 0): Promise<Song[]> {
    try {
      return await subsonicApi.getSongsByGenre(genre, count, offset)
    } catch (e) {
      console.error('Failed to fetch songs by genre:', e)
      return []
    }
  }

  // ==================== 播放列表 ====================

  async function fetchPlaylists() {
    if (isLoadingPlaylists.value) return
    isLoadingPlaylists.value = true

    try {
      playlists.value = await subsonicApi.getPlaylists()
    } catch (e) {
      console.error('Failed to fetch playlists:', e)
    } finally {
      isLoadingPlaylists.value = false
    }
  }

  async function fetchPlaylist(id: string): Promise<PlaylistWithSongs | null> {
    if (playlistCache.value.has(id)) {
      return playlistCache.value.get(id)!
    }

    try {
      const playlist = await subsonicApi.getPlaylist(id)
      playlistCache.value.set(id, playlist)
      return playlist
    } catch (e) {
      console.error('Failed to fetch playlist:', e)
      return null
    }
  }

  async function createPlaylist(name: string, songIds?: string[]): Promise<Playlist | null> {
    try {
      const playlist = await subsonicApi.createPlaylist(name, songIds)
      await fetchPlaylists()
      return playlist
    } catch (e) {
      console.error('Failed to create playlist:', e)
      return null
    }
  }

  async function deletePlaylist(id: string): Promise<boolean> {
    try {
      await subsonicApi.deletePlaylist(id)
      playlistCache.value.delete(id)
      await fetchPlaylists()
      return true
    } catch (e) {
      console.error('Failed to delete playlist:', e)
      return false
    }
  }

  async function addToPlaylist(playlistId: string, songIds: string[]): Promise<boolean> {
    try {
      await subsonicApi.updatePlaylist(playlistId, { songIdToAdd: songIds })
      playlistCache.value.delete(playlistId)
      return true
    } catch (e) {
      console.error('Failed to add to playlist:', e)
      return false
    }
  }

  // ==================== 收藏 ====================

  async function fetchStarred() {
    try {
      starred.value = await subsonicApi.getStarred2()
      // 更新收藏 ID 集合
      starredIds.value.artists = new Set(starred.value.artist?.map(a => a.id) || [])
      starredIds.value.albums = new Set(starred.value.album?.map(a => a.id) || [])
      starredIds.value.songs = new Set(starred.value.song?.map(s => s.id) || [])
    } catch (e) {
      console.error('Failed to fetch starred:', e)
    }
  }

  function isStarred(id: string, type: 'song' | 'album' | 'artist'): boolean {
    if (type === 'song') return starredIds.value.songs.has(id)
    if (type === 'album') return starredIds.value.albums.has(id)
    return starredIds.value.artists.has(id)
  }

  async function star(id: string, type: 'song' | 'album' | 'artist'): Promise<boolean> {
    try {
      if (type === 'song') {
        await subsonicApi.star(id)
        starredIds.value.songs.add(id)
      } else if (type === 'album') {
        await subsonicApi.star(undefined, id)
        starredIds.value.albums.add(id)
      } else {
        await subsonicApi.star(undefined, undefined, id)
        starredIds.value.artists.add(id)
      }
      return true
    } catch (e) {
      console.error('Failed to star:', e)
      return false
    }
  }

  async function unstar(id: string, type: 'song' | 'album' | 'artist'): Promise<boolean> {
    try {
      if (type === 'song') {
        await subsonicApi.unstar(id)
        starredIds.value.songs.delete(id)
      } else if (type === 'album') {
        await subsonicApi.unstar(undefined, id)
        starredIds.value.albums.delete(id)
      } else {
        await subsonicApi.unstar(undefined, undefined, id)
        starredIds.value.artists.delete(id)
      }
      return true
    } catch (e) {
      console.error('Failed to unstar:', e)
      return false
    }
  }

  async function toggleStar(id: string, type: 'song' | 'album' | 'artist'): Promise<boolean> {
    if (isStarred(id, type)) {
      return await unstar(id, type)
    } else {
      return await star(id, type)
    }
  }

  // ==================== 搜索 ====================

  async function search(query: string, options: {
    artistCount?: number
    albumCount?: number
    songCount?: number
  } = {}): Promise<SearchResult3> {
    try {
      return await subsonicApi.search3(query, {
        artistCount: options.artistCount || 20,
        albumCount: options.albumCount || 20,
        songCount: options.songCount || 50
      })
    } catch (e) {
      console.error('Failed to search:', e)
      return {}
    }
  }

  // ==================== 随机歌曲 ====================

  async function fetchRandomSongs(size = 50): Promise<Song[]> {
    try {
      return await subsonicApi.getRandomSongs(size)
    } catch (e) {
      console.error('Failed to fetch random songs:', e)
      return []
    }
  }

  // ==================== 歌曲列表 / 艺术家列表 ====================

  async function fetchSongsList(type: 'frequent' | 'mostPlayed' | 'recent' | 'recentlyPlayed', size = 10): Promise<Song[]> {
    try {
      return await subsonicApi.getSongsList(type, size)
    } catch (e) {
      console.error('Failed to fetch songs list:', e)
      return []
    }
  }

  async function fetchArtistList(type: 'frequent' | 'mostPlayed' | 'recent' | 'recentlyPlayed', size = 10): Promise<Artist[]> {
    try {
      return await subsonicApi.getArtistList(type, size)
    } catch (e) {
      console.error('Failed to fetch artist list:', e)
      return []
    }
  }

  // ==================== 清除缓存 ====================

  function clearCache() {
    artistCache.value.clear()
    albumCache.value.clear()
    playlistCache.value.clear()
  }

  return {
    // 状态
    artistIndexes,
    albums,
    genres,
    playlists,
    starred,
    starredIds,
    
    // 加载状态
    isLoadingArtists,
    isLoadingAlbums,
    isLoadingGenres,
    isLoadingPlaylists,
    
    // 方法
    fetchArtists,
    fetchArtist,
    fetchAlbums,
    fetchAlbum,
    fetchGenres,
    fetchSongsByGenre,
    fetchPlaylists,
    fetchPlaylist,
    createPlaylist,
    deletePlaylist,
    addToPlaylist,
    fetchStarred,
    isStarred,
    star,
    unstar,
    toggleStar,
    search,
    fetchRandomSongs,
    fetchSongsList,
    fetchArtistList,
    clearCache
  }
})

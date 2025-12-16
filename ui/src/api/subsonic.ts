/**
 * Subsonic/OpenSubsonic API Client
 * 基于 OpenSubsonic 规范实现的 API 客户端
 */

import { md5 } from '../utils/crypto'

export interface SubsonicConfig {
  server: string
  username: string
  password: string
  clientName?: string
  apiVersion?: string
}

export interface SubsonicResponse<T = unknown> {
  'subsonic-response': {
    status: 'ok' | 'failed'
    version: string
    type: string
    serverVersion: string
    openSubsonic: boolean
    error?: {
      code: number
      message: string
    }
  } & T
}

// 基础类型定义
export interface Artist {
  id: string
  name: string
  coverArt?: string
  albumCount?: number
  artistImageUrl?: string
  starred?: string
  userRating?: number
}

export interface ArtistWithAlbums extends Artist {
  album: Album[]
}

export interface Album {
  id: string
  name: string
  artist?: string
  artistId?: string
  coverArt?: string
  songCount?: number
  duration?: number
  playCount?: number
  created?: string
  starred?: string
  year?: number
  genre?: string
}

export interface AlbumWithSongs extends Album {
  song: Song[]
}

export interface Song {
  id: string
  parent?: string
  isDir: boolean
  title: string
  album?: string
  albumId?: string
  artist?: string
  artistId?: string
  track?: number
  year?: number
  genre?: string
  coverArt?: string
  size?: number
  contentType?: string
  suffix?: string
  transcodedContentType?: string
  transcodedSuffix?: string
  duration?: number
  bitRate?: number
  path?: string
  playCount?: number
  discNumber?: number
  created?: string
  starred?: string
  userRating?: number
}

export interface Playlist {
  id: string
  name: string
  comment?: string
  owner?: string
  public?: boolean
  songCount: number
  duration: number
  created: string
  changed: string
  coverArt?: string
}

export interface PlaylistWithSongs extends Playlist {
  entry: Song[]
}

export interface Genre {
  songCount: number
  albumCount: number
  value: string
}

export interface MusicFolder {
  id: string
  name: string
}

export interface Index {
  name: string
  artist: Artist[]
}

export interface ArtistInfo {
  biography?: string
  musicBrainzId?: string
  lastFmUrl?: string
  smallImageUrl?: string
  mediumImageUrl?: string
  largeImageUrl?: string
  similarArtist?: Artist[]
}

export interface SearchResult3 {
  artist?: Artist[]
  album?: Album[]
  song?: Song[]
}

export interface PlayQueue {
  entry: Song[]
  current?: string
  position?: number
  username: string
  changed: string
  changedBy: string
}

class SubsonicAPI {
  private config: SubsonicConfig | null = null

  configure(config: SubsonicConfig) {
    this.config = {
      ...config,
      clientName: config.clientName || 'RhythmDesk',
      apiVersion: config.apiVersion || '1.16.1'
    }
  }

  isConfigured(): boolean {
    return this.config !== null
  }

  getConfig(): SubsonicConfig | null {
    return this.config
  }

  private generateAuthParams(): URLSearchParams {
    if (!this.config) {
      throw new Error('API not configured')
    }

    const salt = Math.random().toString(36).substring(2, 12)
    const token = md5(this.config.password + salt)

    return new URLSearchParams({
      u: this.config.username,
      t: token,
      s: salt,
      v: this.config.apiVersion!,
      c: this.config.clientName!,
      f: 'json'
    })
  }

  private async request<T>(
    endpoint: string,
    params: Record<string, string | number | boolean | undefined> = {},
    method: 'GET' | 'POST' = 'GET'
  ): Promise<T> {
    if (!this.config) {
      throw new Error('API not configured')
    }

    const authParams = this.generateAuthParams()
    
    // 添加额外参数
    Object.entries(params).forEach(([key, value]) => {
      if (value !== undefined) {
        authParams.append(key, String(value))
      }
    })

    const url = `${this.config.server}/rest/${endpoint}?${authParams.toString()}`

    const response = await fetch(url, {
      method,
      headers: {
        'Accept': 'application/json'
      }
    })

    if (!response.ok) {
      throw new Error(`HTTP error! status: ${response.status}`)
    }

    const data: SubsonicResponse<T> = await response.json()

    if (data['subsonic-response'].status === 'failed') {
      const error = data['subsonic-response'].error
      throw new Error(error?.message || 'Unknown error')
    }

    return data['subsonic-response'] as T
  }

  // ==================== System ====================
  
  async ping(): Promise<boolean> {
    try {
      await this.request('ping')
      return true
    } catch {
      return false
    }
  }

  // ==================== Browsing ====================

  async getMusicFolders(): Promise<MusicFolder[]> {
    const response = await this.request<{ musicFolders: { musicFolder: MusicFolder[] } }>('getMusicFolders')
    return response.musicFolders?.musicFolder || []
  }

  async getIndexes(musicFolderId?: string): Promise<Index[]> {
    const response = await this.request<{ indexes: { index: Index[] } }>('getIndexes', { musicFolderId })
    return response.indexes?.index || []
  }

  async getArtists(musicFolderId?: string): Promise<Index[]> {
    const response = await this.request<{ artists: { index: Index[] } }>('getArtists', { musicFolderId })
    return response.artists?.index || []
  }

  async getArtist(id: string): Promise<ArtistWithAlbums> {
    const response = await this.request<{ artist: ArtistWithAlbums }>('getArtist', { id })
    return response.artist
  }

  async getAlbum(id: string): Promise<AlbumWithSongs> {
    const response = await this.request<{ album: AlbumWithSongs }>('getAlbum', { id })
    return response.album
  }

  async getSong(id: string): Promise<Song> {
    const response = await this.request<{ song: Song }>('getSong', { id })
    return response.song
  }

  async getArtistInfo(id: string, count?: number, includeNotPresent?: boolean): Promise<ArtistInfo> {
    const response = await this.request<{ artistInfo2: ArtistInfo }>('getArtistInfo2', {
      id,
      count,
      includeNotPresent
    })
    return response.artistInfo2
  }

  async getTopSongs(artist: string, count?: number): Promise<Song[]> {
    const response = await this.request<{ topSongs: { song: Song[] } }>('getTopSongs', { artist, count })
    return response.topSongs?.song || []
  }

  async getSimilarSongs(id: string, count?: number): Promise<Song[]> {
    const response = await this.request<{ similarSongs2: { song: Song[] } }>('getSimilarSongs2', { id, count })
    return response.similarSongs2?.song || []
  }

  async getGenres(): Promise<Genre[]> {
    const response = await this.request<{ genres: { genre: Genre[] } }>('getGenres')
    return response.genres?.genre || []
  }

  // ==================== Album/Song Lists ====================

  async getAlbumList2(
    type: 'random' | 'newest' | 'highest' | 'frequent' | 'recent' | 'alphabeticalByName' | 'alphabeticalByArtist' | 'starred' | 'byYear' | 'byGenre',
    options: {
      size?: number
      offset?: number
      fromYear?: number
      toYear?: number
      genre?: string
      musicFolderId?: string
    } = {}
  ): Promise<Album[]> {
    const response = await this.request<{ albumList2: { album: Album[] } }>('getAlbumList2', {
      type,
      ...options
    })
    return response.albumList2?.album || []
  }

  async getRandomSongs(size?: number, genre?: string, fromYear?: number, toYear?: number, musicFolderId?: string): Promise<Song[]> {
    const response = await this.request<{ randomSongs: { song: Song[] } }>('getRandomSongs', {
      size,
      genre,
      fromYear,
      toYear,
      musicFolderId
    })
    return response.randomSongs?.song || []
  }

  async getSongsByGenre(genre: string, count?: number, offset?: number, musicFolderId?: string): Promise<Song[]> {
    const response = await this.request<{ songsByGenre: { song: Song[] } }>('getSongsByGenre', {
      genre,
      count,
      offset,
      musicFolderId
    })
    return response.songsByGenre?.song || []
  }

  async getStarred2(musicFolderId?: string): Promise<{ artist: Artist[]; album: Album[]; song: Song[] }> {
    const response = await this.request<{ starred2: { artist?: Artist[]; album?: Album[]; song?: Song[] } }>('getStarred2', { musicFolderId })
    return {
      artist: response.starred2?.artist || [],
      album: response.starred2?.album || [],
      song: response.starred2?.song || []
    }
  }

  // ==================== Media Annotation ====================

  async star(id?: string, albumId?: string, artistId?: string): Promise<void> {
    await this.request('star', { id, albumId, artistId }, 'POST')
  }

  async unstar(id?: string, albumId?: string, artistId?: string): Promise<void> {
    await this.request('unstar', { id, albumId, artistId }, 'POST')
  }

  async setRating(id: string, rating: number): Promise<void> {
    await this.request('setRating', { id, rating }, 'POST')
  }

  async scrobble(id: string, time?: number, submission?: boolean): Promise<void> {
    await this.request('scrobble', { id, time, submission }, 'POST')
  }

  // ==================== Playlists ====================

  async getPlaylists(username?: string): Promise<Playlist[]> {
    const response = await this.request<{ playlists: { playlist: Playlist[] } }>('getPlaylists', { username })
    return response.playlists?.playlist || []
  }

  async getPlaylist(id: string): Promise<PlaylistWithSongs> {
    const response = await this.request<{ playlist: PlaylistWithSongs }>('getPlaylist', { id })
    return response.playlist
  }

  async createPlaylist(name: string, songId?: string[]): Promise<Playlist> {
    const params: Record<string, string> = { name }
    if (songId) {
      songId.forEach((id, i) => {
        params[`songId[${i}]`] = id
      })
    }
    const response = await this.request<{ playlist: Playlist }>('createPlaylist', params, 'POST')
    return response.playlist
  }

  async updatePlaylist(
    playlistId: string,
    options: {
      name?: string
      comment?: string
      public?: boolean
      songIdToAdd?: string[]
      songIndexToRemove?: number[]
    }
  ): Promise<void> {
    const params: Record<string, string | boolean | undefined> = {
      playlistId,
      name: options.name,
      comment: options.comment,
      public: options.public
    }
    options.songIdToAdd?.forEach((id, i) => {
      params[`songIdToAdd[${i}]`] = id
    })
    options.songIndexToRemove?.forEach((idx, i) => {
      params[`songIndexToRemove[${i}]`] = String(idx)
    })
    await this.request('updatePlaylist', params, 'POST')
  }

  async deletePlaylist(id: string): Promise<void> {
    await this.request('deletePlaylist', { id }, 'POST')
  }

  // ==================== Bookmarks / Play Queue ====================

  async savePlayQueue(id: string[], current?: string, position?: number): Promise<void> {
    const params: Record<string, string | number | undefined> = { current, position }
    id.forEach((songId, i) => {
      params[`id[${i}]`] = songId
    })
    await this.request('savePlayQueue', params, 'POST')
  }

  async getPlayQueue(): Promise<PlayQueue | null> {
    try {
      const response = await this.request<{ playQueue: PlayQueue }>('getPlayQueue')
      return response.playQueue || null
    } catch {
      return null
    }
  }

  // ==================== Searching ====================

  async search3(
    query: string,
    options: {
      artistCount?: number
      artistOffset?: number
      albumCount?: number
      albumOffset?: number
      songCount?: number
      songOffset?: number
      musicFolderId?: string
    } = {}
  ): Promise<SearchResult3> {
    const response = await this.request<{ searchResult3: SearchResult3 }>('search3', {
      query,
      ...options
    })
    return response.searchResult3 || {}
  }

  // ==================== Media Retrieval ====================

  getCoverArtUrl(id: string, size?: number): string {
    if (!this.config) {
      return ''
    }
    const params = this.generateAuthParams()
    params.append('id', id)
    if (size) {
      params.append('size', String(size))
    }
    return `${this.config.server}/rest/getCoverArt?${params.toString()}`
  }

  getStreamUrl(id: string, options: {
    maxBitRate?: number
    format?: string
    timeOffset?: number
    estimateContentLength?: boolean
  } = {}): string {
    if (!this.config) {
      return ''
    }
    const params = this.generateAuthParams()
    params.append('id', id)
    if (options.maxBitRate) params.append('maxBitRate', String(options.maxBitRate))
    if (options.format) params.append('format', options.format)
    if (options.timeOffset) params.append('timeOffset', String(options.timeOffset))
    if (options.estimateContentLength) params.append('estimateContentLength', 'true')
    return `${this.config.server}/rest/stream?${params.toString()}`
  }

  // ==================== Library Scanning ====================

  async startScan(): Promise<{ scanning: boolean; count?: number }> {
    const response = await this.request<{ scanStatus: { scanning: boolean; count?: number } }>('startScan')
    return response.scanStatus
  }

  async getScanStatus(): Promise<{ scanning: boolean; count?: number }> {
    const response = await this.request<{ scanStatus: { scanning: boolean; count?: number } }>('getScanStatus')
    return response.scanStatus
  }

  // ==================== OpenSubsonic Extended ====================

  async getSongsList(type: 'frequent' | 'mostPlayed' | 'recent' | 'recentlyPlayed', size?: number): Promise<Song[]> {
    const response = await this.request<{ songList: { song: Song[] } }>('getSongsList', { type, size })
    return response.songList?.song || []
  }

  async getArtistList(type: 'frequent' | 'mostPlayed' | 'recent' | 'recentlyPlayed', size?: number): Promise<Artist[]> {
    const response = await this.request<{ artistList: { artist: Artist[] } }>('getArtistList', { type, size })
    return response.artistList?.artist || []
  }
}

export const subsonicApi = new SubsonicAPI()
export default subsonicApi

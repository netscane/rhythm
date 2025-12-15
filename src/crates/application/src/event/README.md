# Event Handling Documentation

## Library Events
- ItemAdded(LibraryItem) - 新增库项目
  + Song领域需要添加AudioSource到Song
- ItemRemoved(LibraryItem) - 移除库项目
  + Song领域需要从Song中移除AudioSource
- LibraryDeleted(u64) - 删除库
  + Song领域需要删除该库下的所有

## Song Events
- SongCreated(SongCreatedEvent) - 新增歌曲
  + Genre领域
    - 将Song添加到该Genre中
  + Artist领域
    - 将Song添加到Artist的songs列表中
  + Album领域
    - 将Song添加到Album的tracks列表中
- SongDeleted(SongDeletedEvent) - 删除歌曲
  + Genre领域
    - 将Song从Genre中删除
  + Artist领域
    - 从Artist的songs列表中移除Song
- AudioSourceAdded(AudioSourceAddedEvent) - 添加音频源
  + Album领域
    - 将AudioSource添加到Album的tracks列表中
- AudioSourceRemoved(AudioSourceRemovedEvent) - 移除音频源
  + Album领域
    - 从Album的tracks列表中移除AudioSource

## Artist Events
- ArtistGenreUpdated(ArtistGenreUpdated) - 艺术家流派更新
  + Genre领域
    - 更新记录了Artist的Genre中Artist
- ArtistAdded(Artist) - 新增艺术家
  + Genre领域
    - 将Artist添加到Genre中
- ArtistRemoved(Artist) - 移除艺术家
  + Genre领域
    - 将Artist从Genre中移除

## Album Events
- AlbumCreated(AlbumCreated) - 新增专辑
  + Genre领域
    - 将Album添加到Genre中
  + Artist领域
    - 将album添加到artist拥有的album中
- AlbumGenreUpdated(AlbumGenreUpdated) - 专辑流派更新
  + Genre领域
    - 更新album的genre
- AlbumArtistUpdated(AlbumArtistUpdated) - 专辑艺术家更新
  + Artist领域
    - 更新album的artist
- AlbumDeleted(AlbumDeleted) - 专辑音轨删除
  + Genre领域
    - 从genre中移除album
  + Artist领域
    - 从artist中移除album

## User Events
- UserAdded(User) - 新增用户
  + Annotation领域
    - 初始化用户的Annotation数据
  + Playlist领域
    - 初始化用户的播放列表
- UserDeleted(u64) - 删除用户
  + Annotation领域
    - 删除用户的Annotation数据
  + Playlist领域
    - 删除用户创建的播放列表
- UserUpdated(User) - 更新用户
  + Playlist领域
    - 更新用户名
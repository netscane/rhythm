# Rhythm

A personal music streaming server written in Rust, implementing the Subsonic/OpenSubsonic API protocol.

## Features

- **Subsonic API Compatible** - Works with popular clients like Symfonium, Substreamer, Sublime Music, etc.
- **Media Library Management** - Scan local filesystem or SMB network shares
- **Audio Metadata Extraction** - Automatic parsing of title, artist, album, year, genre, etc.
- **Cover Art Detection** - Smart matching of cover images (cover.*, folder.*, front.*, etc.)
- **Real-time Transcoding** - FFmpeg-based transcoding with caching support
- **Modern Web UI** - Built with Vue 3 + TypeScript
- **DDD Architecture** - Clean separation of domain, application, infrastructure layers

## Tech Stack

### Backend (Rust)
| Component | Technology |
|-----------|------------|
| Web Framework | Actix-web 4 |
| Database ORM | SeaORM (PostgreSQL) |
| Authentication | JWT + Bcrypt |
| Encryption | AES-256-GCM |
| Transcoding | FFmpeg |

### Frontend (Vue.js)
| Component | Technology |
|-----------|------------|
| Framework | Vue 3.5 |
| Build Tool | Vite 6 |
| State Management | Pinia 3 |
| Router | Vue Router 4 |
| Language | TypeScript |

## Quick Start

### Prerequisites

- Rust 1.70+
- PostgreSQL 14+
- FFmpeg (for transcoding)
- Node.js 18+ & pnpm (for UI)

### Installation

1. **Clone the repository**
   ```bash
   git clone https://github.com/yourname/rhythm.git
   cd rhythm
   ```

2. **Configure the application**
   ```bash
   cp config.toml.example config.toml
   # Edit config.toml with your settings
   ```

3. **Setup database**
   ```bash
   # Create PostgreSQL database
   createdb rhythm
   
   # Run migrations
   cargo run -p migration
   ```

4. **Build and run the server**
   ```bash
   cargo build --release
   ./target/release/rhythm
   ```

   On first startup, an admin user will be automatically created with a random password. Check the console output:
   ```
   ===========================================
     Admin user created successfully!
     Username: admin
     Password: <random_password>
     Please change the password after login!
   ===========================================
   ```

5. **Build the Web UI** (optional)
   ```bash
   cd ui
   pnpm install
   pnpm build
   ```

6. **Access the application**
   - Web UI: http://localhost:5533/app
   - Subsonic API: http://localhost:5533/rest/

## Configuration

Edit `config.toml` to customize your setup:

```toml
# Database connection
database_url = "postgresql://user:pass@localhost:5432/rhythm"

# JWT authentication
jwt_secret_key = "your_secret_key"
jwt_expire_secs = 3600

# Password encryption key (for AES-256-GCM encrypted storage, supports Subsonic token auth)
password_encryption_key = "your_password_encryption_key_here"

# Cover art filename wildcards (sorted by priority, higher priority first)
# Formats: "name.*" matches filename, "*.ext" matches extension, "*" matches all
cover_art_wildcards = ["cover.*", "folder.*", "front.*", "album.*", "albumart.*", "*"]

# Music library configuration (auto-synced on every startup)
# Libraries defined here will be created if not exist
# Supports multiple libraries with local or SMB paths
[[music_folders]]
name = "Music"
protocol = "local"  # "local" or "smb"
path = "/path/to/your/music"

# Add multiple music folders as needed
# [[music_folders]]
# name = "NAS Music"
# protocol = "smb"
# path = "//server/share/music"

# Server settings
[server]
host = "0.0.0.0"
port = 5533
ui_path = "ui/dist"
ui_base_path = "/app"

# Transcoding settings
[transcoding]
ffmpeg_path = "ffmpeg"
default_format = "mp3"
default_bit_rate = 192
cache_enabled = true
cache_ttl_secs = 2592000  # 30 days
chunk_size = 65536
lossless_formats = ["flac", "wav", "aiff", "ape", "dsf", "dff", "wv"]

# Cache settings
[cache]
data_dir = "./data/cache"
ttl_secs = 604800  # 7 days
```

## Project Structure

```
src/crates/
├── domain/       # Domain layer - Core business logic
├── application/  # Application layer - Use cases (CQRS)
├── infra/        # Infrastructure - Database, storage, etc.
├── server/       # Presentation - HTTP API
├── model/        # Data transfer objects
└── migration/    # Database migrations

ui/               # Vue.js frontend
```

## Subsonic API Support

Rhythm implements the Subsonic API, supporting:

- **Browsing**: Artists, Albums, Songs, Genres, Folders
- **Searching**: search2, search3
- **Media Annotation**: star, unstar, setRating, scrobble
- **Playlists**: create, update, delete, getPlaylists
- **Play Queue**: save, get
- **User Management**: getUser, createUser, updateUser
- **Streaming**: stream, download with transcoding support
- **Cover Art**: getCoverArt with caching

## License

MIT License

# bsky_tui
Bluesky client for the terminal

![](./assets/splash.png)

## Overview

bsky_tui is a terminal-based client for Bluesky. Through a command-line interface, you can access the main features of Bluesky. Using a Text User Interface (TUI), you can efficiently browse timelines, create posts, like, repost, reply, and more using just your keyboard.

## Features

- View and browse home timeline
- Display profile pictures and post image attachments in supported terminals
- View and browse notifications
- Search for posts
- Search for users
- View profiles, profile posts/replies/media/likes, feeds, lists, and starter packs
- Follow/unfollow users and browse followers/following
- Mute, block, and report profiles or posts with confirmation
- Switch between Following, Discover, saved, pinned, and searched custom feeds
- Save posts privately and browse the cursor-paginated Bookmarks feed
- Create image, link-card, quote, and multi-post thread posts
- Set post languages, content warnings, and reply controls
- View post threads and reply trees
- View quote posts, external link cards, and video/GIF metadata
- Open URLs, mentions, hashtags, external links, and videos
- Browse users who liked or reposted a post, and posts quoting it
- Create new posts
- Reply to posts
- Like/unlike posts
- Repost/unrepost posts
- Open posts in browser
- Browse, create, edit, save, and moderate lists
- Browse, create, edit, and join Starter Packs; view trends and suggested follows
- Read and send direct messages through the dedicated Bluesky chat service
- Manage muted words/threads, labelers, content labels, and advanced thread safety actions
- Register and switch accounts, with configurable colors, images, dates, language, and shortcuts

The supported SDK and lexicon baseline is documented in
[`PROTOCOL_COMPATIBILITY.md`](PROTOCOL_COMPATIBILITY.md).

## Installation

### Build from source

```bash
git clone https://github.com/ksk001100/bsky_tui
cd bsky_tui
cargo install --path .
```

### Install using Cargo

```bash
cargo install bsky_tui
```

## Configuration

You need to generate a configuration file on first launch:

```bash
bsky_tui config
```

This will generate a configuration file at:
- Linux/macOS: `~/.config/bsky_tui/config.toml`
- Windows: `~\AppData\Roaming\bsky_tui\config.toml`

Edit the generated configuration file and set the following required fields:

```toml
identifier = "your.email@example.com" # Your Bluesky account email or handle
service_url = "https://bsky.social" # Your PDS/service URL
skip_splash = false               # Whether to skip the splash screen (optional)
splash_path = ""                  # Path to a custom splash screen (optional)

[ui]
show_images = true
date_format = "%Y-%m-%d %H:%M"
language = "auto"                 # Used for posts without an explicit !lang directive
accent_color = "blue"
auto_refresh_seconds = 60       # 0 disables background refresh

[ui.keybindings]
# action_menu = "ctrl+p"
# move_down = "ctrl+j"
open_lists = "g"
open_dm = "d"
open_moderation = ";"
open_settings = ","
```

Additional accounts can be registered in the Settings panel or in `config.toml`:

```toml
active_account = "alice.example"

[[accounts]]
identifier = "alice.example"
service_url = "https://bsky.social"
```

App Passwords remain separate in the OS credential store and are keyed by account identifier.

Create an App Password in Bluesky under **Settings → Privacy and security → App passwords**,
then store it in the operating system's credential store. Input is hidden and confirmed before
the credential is saved:

```bash
bsky_tui credentials set
```

This uses macOS Keychain, the Linux Secret Service (for example GNOME Keyring or KWallet), or
Windows Credential Manager. The App Password is not stored in `config.toml` or in the UI state.
Existing `email` entries remain supported as an alias for `identifier`; remove any old `password`
entry from the file. The credential is read only during authentication and its in-process buffer
is cleared after use.

On a headless Linux system without Secret Service, the environment variable remains available as
a fallback:

```bash
BSKY_TUI_APP_PASSWORD="xxxx-xxxx-xxxx-xxxx" bsky_tui
```

Delete the saved credential with `bsky_tui credentials delete`.

## Usage

```bash
# Show help
bsky_tui --help

# Generate config file
bsky_tui config

# Save the App Password in the OS keyring
bsky_tui credentials set

# Launch the application
bsky_tui
```

Image rendering uses the best protocol detected for the current terminal. Kitty-compatible
terminals use the Kitty graphics protocol; unsupported terminals fall back to a compatible
text-cell rendering mode.

## Keybindings

### Common
- `q`, `Esc`: Cancel, close, or go back (they do not quit from a main tab)
- `Ctrl+c`: Exit the application
- `Tab`: Switch tabs (Home → Notifications → Messages → Explore → Home)
- `?`, `F1`: Show help (also available in threads and profiles)
- `u`: Search for users
- `a`: Open the selected post author profile (where available)
- `m`, `B`, `!`: Mute/unmute, block/unblock, or report (with confirmation)
- `D`: Delete the selected post when it belongs to you (with confirmation)
- `X`: Quote the selected post
- `g`: Open Lists and Starter Packs
- `d`: Jump directly to the Messages tab
- `;`: Open Moderation & Safety
- `,`: Open Settings & Accounts
- `:`: Open the action menu
- Mouse wheel: Move the current selection
- Left click on the tab bar: Switch tabs
- `PageUp`, `PageDown`: Move half a page within the current list
- `Home`, `End`: Move to the first or last item
- `y`, `Y`, `Alt+y`: Copy post text, web URL, or AT URI + author DID using OSC 52

### Home Tab
- `j`, `Down`, `Ctrl+n`: Scroll down
- `k`, `Up`, `Ctrl+p`: Scroll up
- `h`, `Left`: Previous page
- `l`, `Right`: Next page
- `[`: Previous API page
- `]`: Next API page
- `F5`: Reload timeline
- `c`: Open the feed selector
- `n`: New post
- `r`: Reply to selected post
- `Ctrl+l`: Like/unlike
- `Ctrl+r`: Repost/unrepost
- `Enter`: Open selected post thread
- `i`, `Space`: Open attached images
- `o`: Open selected post in browser
- `e`: Open an external link or video embed
- `f`: Select a URL, mention, or hashtag from the post
- `L`, `R`, `Q`: Show Likes, Reposts, or Quotes
- `/`: Switch to search mode

### Notifications Tab
- `j`, `Down`, `Ctrl+n`: Scroll down
- `k`, `Up`, `Ctrl+p`: Scroll up
- `F5`: Reload notifications
- `1`: Cycle notification reason filter
- `2`: Cycle sender filter (all, following, others)
- `3`: Cycle read-state filter (all, unread, read)
- `p`: Open notification and activity settings for the selected sender
- `f`: Follow/unfollow the selected sender
- `L`: Like the selected sender's latest post (does nothing if already liked)
- `a`: Open the selected sender's profile
- `Enter`: Open the related post thread inside the TUI
- `o`: Open the related post in the browser
- `h`, `Left`: Previous page
- `l`, `Right`: Next page
- `[`: Previous API page
- `]`: Next API page
- `/`: Switch to search mode

Notifications hydrate their related posts and show the author, post text, relative time,
embed summary, attached images or link/video thumbnails, and interaction counts directly
in the list. Sensitive-media visibility follows the same moderation rules as the timeline.

### Messages Tab
- `j`, `Down`, `Ctrl+n`: Move down
- `k`, `Up`, `Ctrl+p`: Move up
- `Enter`, `o`: Open the selected conversation
- `n`: Start a conversation by handle or DID
- `w`: Write a text message (1,000 characters maximum)
- `Space`: Mute or unmute the selected conversation
- `r`: Report the selected conversation or message
- `b`: Block the selected participant
- `F5`: Reload conversations or the open conversation
- `q`, `Esc`: Return from a conversation to the conversation list

### Explore Tab
- Before searching, the tab shows suggested topics and accounts
- `Enter`, `o`: Search a suggested topic or open a suggested profile
- `Esc`: Return from search results to Explore
- `j`, `Down`, `Ctrl+n`: Scroll down
- `k`, `Up`, `Ctrl+p`: Scroll up
- `h`, `Left`: Previous page
- `l`, `Right`: Next page
- `[`: Previous API page
- `]`: Next API page
- `F5`: Reload search results
- `r`: Reply to selected post
- `Ctrl+l`: Like/unlike
- `Ctrl+r`: Repost/unrepost
- `Enter`: Open selected post thread
- `i`, `Space`: Open attached images
- `o`: Open selected post in browser
- `e`: Open an external link or video embed
- `f`: Select a URL, mention, or hashtag from the post
- `L`, `R`, `Q`: Show Likes, Reposts, or Quotes
- `/`: Switch to search mode

### Thread Viewer
- `j`, `Down`, `Ctrl+n`: Move down through the thread
- `k`, `Up`, `Ctrl+p`: Move up through the thread
- `o`: Open the selected post in browser
- `a`: Open the selected post author profile
- `i`, `Space`: Open attached images
- `e`: Open an external link or video embed
- `f`: Select a URL, mention, or hashtag from the post
- `L`, `R`, `Q`: Show Likes, Reposts, or Quotes
- `H`: Hide or unhide the selected reply when the thread is yours
- `M`: Mute or unmute the thread
- `Ctrl+d`: Detach a selected quote of your own post
- `q`, `Esc`: Close the thread

### Profile Viewer
- `h`, `Left`: Previous profile section
- `l`, `Right`: Next profile section
- `j`, `Down`, `Ctrl+n`: Move down
- `k`, `Up`, `Ctrl+p`: Move up
- `F`: Follow/unfollow the profile
- `m`, `B`, `!`: Mute/unmute, block/unblock, or report the profile
- `g`: Show followers
- `G`: Show following
- `a`: Open the selected post author profile
- `t`: Open the selected post thread
- `i`, `Space`: Open attached images when the selected item is a post
- `Enter`, `o`: Open the selected post, feed, list, or starter pack
- `q`, `Esc`: Close the profile

### Facet and Interaction Lists
- `j`, `Down`: Move down
- `k`, `Up`: Move up
- `Enter`, `o`: Open the selected item in browser
- `q`, `Esc`: Close the list

### Feed Selector
- `j`, `Down`: Move down
- `k`, `Up`: Move up
- `Enter`: Switch to the selected feed
- `/`: Search custom feeds
- `s`: Save or unsave a custom feed
- `!`: Report a custom feed with a selected reason
- `q`, `Esc`: Close the selector

### Lists, Starter Packs, Direct Messages, Moderation, and Settings

- `1`–`6`: Switch between Lists, Starter Packs, Discover, DMs, Moderation, and Settings
- `j`, `Down` / `k`, `Up`: Move selection
- `Enter`, `o`: Open the selected item
- `n`, `a`, `e`, `x`: Create, add member, edit, or delete where supported
- `s`: Save a curational list or subscribe/unsubscribe a moderation list
- `f`: Use a curational list as the home feed
- `J`: Join the open Starter Pack by following its members
- `w`: Write a message in an open conversation
- `Space`: Mute or unmute a conversation
- `b`, `r`: Block or report a DM participant/content
- `L`, `l`: Add or toggle a labeler subscription
- `q`, `Esc`: Go to the parent view or close the panel (`Esc` alone closes an active editor)

### Image Viewer
- `h`, `Left`: Previous image
- `l`, `Right`: Next image
- `q`, `Esc`: Close image viewer

### Post/Reply Mode
- `Esc`: Cancel
- `Enter`: Insert a newline
- `Ctrl+s`: Send post/reply
- `Ctrl+v`: Preview the first `!link` card
- `Left`, `Ctrl+b`: Move cursor left
- `Right`, `Ctrl+f`: Move cursor right
- `Ctrl+a`: Move cursor to start
- `Ctrl+e`: Move cursor to end
- `Backspace`, `Ctrl+h`: Delete previous character

Composer directives are placed on their own lines and are removed from the published text:

```text
!image /path/to/image.png | Accessible alt text
!link https://example.com/article
!lang ja,en
!label nudity
!replies followers,mentioned
Post body
---
Second post in the same thread
```

## Protocol and SDK compatibility

This release is built against `atrium-api 0.25.8`, `atrium-xrpc 0.12`,
`atrium-xrpc-client 0.5`, and `bsky-sdk 0.1.24`. The implemented endpoints use
the `app.bsky.*`, `chat.bsky.*`, and `com.atproto.*` lexicons shipped by those
versions. Dependency updates are checked by CI via the committed `Cargo.lock`;
update the versions here whenever the SDK or generated lexicons change.

Operational logs contain only an operation name, timestamp, and success/error
status. Credentials, identifiers, post text, and direct-message bodies are not
passed to the logger. Logs are stored in the platform-local data directory.

- Up to four images and 1 MB per image are accepted; dimensions are used for the aspect ratio.
- `!replies` accepts `everyone`, `none`, or a comma-separated combination of `followers`, `following`, and `mentioned`.
- Separate thread posts with a line containing only `---`.
- `X` pre-fills the internal `!quote AT_URI | CID` directive for the selected post.

### Post/User Search Input Mode
- `Esc`: Cancel
- `Enter`: Execute search
- `Left`, `Ctrl+b`: Move cursor left
- `Right`, `Ctrl+f`: Move cursor right
- `Ctrl+a`: Move cursor to start
- `Ctrl+e`: Move cursor to end
- `Backspace`, `Ctrl+h`: Delete previous character

## Development

### Dependencies

- [crossterm](https://github.com/crossterm-rs/crossterm): Terminal manipulation
- [ratatui](https://github.com/ratatui-org/ratatui): TUI framework
- [tokio](https://github.com/tokio-rs/tokio): Async runtime
- [bsky-sdk](https://github.com/sugyan/bsky-sdk): Bluesky SDK
- [atrium-api](https://github.com/sugyan/atrium): Bluesky API client
- [seahorse](https://github.com/ksk001100/seahorse): CLI framework

### Build

```bash
cargo build
```

### Test

```bash
cargo test
```

## License

See the [LICENSE](LICENSE) file.

## Contributing

1. Fork this repository
2. Create a feature branch (`git checkout -b my-new-feature`)
3. Commit your changes (`git commit -am 'Add some feature'`)
4. Push to the branch (`git push origin my-new-feature`)
5. Create a new Pull Request

## Author

- Keisuke Toyota ([@ksk001100](https://github.com/ksk001100))

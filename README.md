# books-tui

Browse and open your PDF library from the terminal.

![books-tui screenshot](assets/screenshot.png)

- Fuzzy search across your library
- Reading progress from zathura history
- Author, page count, and year via `pdfinfo` (cached)
- Live refresh when PDFs are added or removed
- Linux and macOS

## Install

**Homebrew** (macOS, Linux)

```sh
brew install prakhil-tp/tap/books-tui
```

**Debian / Ubuntu**

```sh
arch=$(dpkg --print-architecture)   # amd64 or arm64
curl -fsSLO "https://github.com/Prakhil-tp/books-tui/releases/download/v0.1.1/books-tui_0.1.1-1_${arch}.deb"
sudo apt install ./books-tui_0.1.1-1_${arch}.deb
```

**Arch Linux**

```sh
curl -fsSLO https://github.com/Prakhil-tp/books-tui/releases/download/v0.1.1/books-tui-bin-0.1.1-1-x86_64.pkg.tar.zst
sudo pacman -U ./books-tui-bin-0.1.1-1-x86_64.pkg.tar.zst
```

Or build it yourself: `git clone https://github.com/Prakhil-tp/books-tui && cd books-tui/packaging/aur && makepkg -si`

**Prebuilt binary** from [Releases](https://github.com/Prakhil-tp/books-tui/releases/latest)

```sh
# x86_64-unknown-linux-musl, aarch64-unknown-linux-musl,
# x86_64-apple-darwin, or aarch64-apple-darwin
target=x86_64-unknown-linux-musl
mkdir -p ~/.local/bin
curl -fsSL "https://github.com/Prakhil-tp/books-tui/releases/latest/download/books-tui-$target.tar.gz" \
  | tar -xz -C ~/.local/bin books-tui
```

**From source** (Rust 1.88+)

```sh
cargo install --git https://github.com/Prakhil-tp/books-tui.git
```

Optional: `zathura` as the reader (offered for install on first run) and
`poppler-utils` for `pdfinfo` metadata.

## Usage

```sh
books-tui                      # open the library
books-tui --books-dir ~/papers # use another directory for this session
books-tui --list               # print PDF paths and exit
```

| Key | Action |
| --- | --- |
| `j` / `k` | move |
| `gg` / `G` | top / bottom |
| `Ctrl-d` / `Ctrl-u` | half page down / up |
| `/` | filter (`Enter` apply, `Esc` cancel) |
| `Enter` / `l` | open |
| `q` | quit |

## Config

Created on first run at `$XDG_CONFIG_HOME/books-tui/config.toml`
(falling back to `~/.config` on Linux, `~/Library/Application Support` on macOS):

```toml
books_dir = "~/books"
reader = "zathura"
```

`reader` may include arguments, e.g. `"zathura --fork"`. Set `NO_COLOR` to
disable colors.

# gadacz

A Terminal User Interface (TUI) for playing audiobooks (and other audio files)

![screen_2022-10-19_14-56](https://user-images.githubusercontent.com/83038443/198891245-b48c511e-d140-4349-8525-2bbf857e13b9.png)

## Features

- remembers the last file (or m4a/m4b chapter) played and position inside it
- displays information from tags
- playback speed control
- bookmarks
- supports m4a/m4b files with chapters
- antispoiler mode (hides number and names of chapters past the currently selected one)

## Requirements

- gstreamer and its plugins

```
# On Arch
sudo pacman -S gstreamer gst-plugins-base gst-plugins-good
```

- Rust

## Installation

Currently there are no provided binaries. Please install from source.

```
git clone https://github.com/rareitems/gadacz
cd gadacz
cargo install --path .
```

Or to build wihout mp4ameta:
```
cargo install --no-default-features --path .
```

## Usage

```
gadacz <path_to_your_audiobook>
```
Press '?' for a complete list of keymaps.

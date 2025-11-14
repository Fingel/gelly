# Gelly

Gelly is [Jellyfin Media Server](https://jellyfin.org/) client focused on music playback.

<a href='https://flathub.org/apps/io.m51.Gelly'>
  <img width='240' alt='Get it on Flathub' src='https://flathub.org/api/badge?locale=en'/>
</a>

<img width="1106" height="844" alt="Screenshot from 2025-11-13 18-28-37" src="https://github.com/user-attachments/assets/ab37e090-5c70-4eec-a365-0da3a9b201e8" />

## Features

- [x] Play music!
- [x] Browse albums and artists
- [x] Simple playlist management
- [x] MPRIS support
- [x] Search
- [x] Remote playlist management
    - [ ] Edit playlists
- [ ] Eye candy

## Installation

### Flatpak

Gelly is available on Flatpak as [io.m51.Gelly](https://flathub.org/apps/io.m51.Gelly)

    flatpak install io.m51.Gelly

### Arch Linux

Gelly is available on the [aur](https://aur.archlinux.org/packages/gelly):

    paru -S gelly

## Development

Gelly leverages [gtk-rs](https://gtk-rs.org/) for the UI and
[gstreamer](https://gstreamer.freedesktop.org/) for playback. Thus you will need
development libraries installed for both GTK and gstreamer to build from source.

Gelly does *not* require any nightly features from Rust.

The [justfile](justfile) contains recipes, simply running the default recipe `just` should be enough
to build and launch Gelly.


## Windows/OSX
Although there isn't anything preventing Gelly from being cross platform, it is not my focus.
I would accept reasonable PRs to enable other platforms.

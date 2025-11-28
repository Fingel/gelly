# Gelly

Gelly is a native, lightweight [Jellyfin Media Server](https://jellyfin.org/) client focused on music playback written in Rust and GTK.

<img width="1035" height="1030" alt="gellynov27" src="https://github.com/user-attachments/assets/b176afc6-c5c5-43b6-8f1c-bfed0b9768e7" />


## Features

- [x] Play music!
- [x] Browse albums and artists
- [x] Simple playlist management
- [x] MPRIS support
- [x] Search
- [x] Remote playlist management
    - [ ] Edit playlists
- [ ] Eye candy
- [ ] Smart Playlists 
    - [x] Shuffle songs 
    - [ ] Most played
- [ ] Offline support

## Installation

### Flatpak

<a href='https://flathub.org/apps/io.m51.Gelly'>
  <img width='240' alt='Get it on Flathub' src='https://flathub.org/api/badge?locale=en'/>
</a>

Gelly is available on Flatpak as [io.m51.Gelly](https://flathub.org/apps/io.m51.Gelly)

    flatpak install io.m51.Gelly

### Arch Linux

Gelly is available on the [aur](https://aur.archlinux.org/packages/gelly):

    paru -S gelly

## Using Self Signed Certificates with Jellyfin

There is currently [an issue with Flatpak](https://gitlab.com/freedesktop-sdk/freedesktop-sdk/-/issues/1905) 
that prevents sandboxed applications from reading the host's certifcate store. This means if you are
hosting Jellyfin on a server with self-signed certificates which you have installed on the system
where you are trying to use the Gelly Flatpak, it will probably fail to connect.

[#15](https://github.com/Fingel/gelly/issues/15) tracks this issue. The workaround for now is to 
use an alternative installation method other than Flatpak ir to connect without TLS. 
I am looking for someone to help test using alternative TLS backends for reqwest 
that might fix this issue.

## Development

Gelly leverages [gtk-rs](https://gtk-rs.org/) for the UI and
[gstreamer](https://gstreamer.freedesktop.org/) for playback. Thus you will need
development libraries installed for both GTK and gstreamer to build from source.

Gelly does *not* require any nightly features from Rust.

The [justfile](justfile) contains recipes, simply running the default recipe `just` should be enough
to build and launch Gelly.

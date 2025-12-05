<p align="center">
    <img width="150" height="150" src="https://github.com/user-attachments/assets/156e5858-a48e-4ea5-a4e4-6fbcd6644dd7" align="center" /><br />
    <strong style="font-size: 26px;">Gelly</strong><br>
    <em>A native, lightweight Jellyfin client for music. Written in Rust and GTK.</em>
    🦀🐧
</p>

## Features

- [x] Browse by album, artist and playlist
- [x] MPRIS support
- [x] Search
- [x] Edit Playlists
- [ ] Eye candy
- [ ] Smart Playlists 
    - [x] Shuffle songs 
    - [ ] Most played
- [ ] Offline support

<img width="933" height="675" alt="gelly-use-1" src="https://github.com/user-attachments/assets/26e1221d-580f-4da1-aaea-6d0efa6030b7" />

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

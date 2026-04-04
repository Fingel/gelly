<p align="center">
    <img width="150" height="150" src="https://github.com/user-attachments/assets/156e5858-a48e-4ea5-a4e4-6fbcd6644dd7" align="center" /><br />
    <br />
    <strong style="font-size: 26px;">Gelly</strong><br>
    <em>A native, lightweight music client for Jellyfin and Subsonic. Built with Rust and GTK.</em>
    🦀🐧
</p>

<img width="873" height="769" alt="Screenshot From 2026-04-03 21-02-02" src="https://github.com/user-attachments/assets/fa25aece-ebe7-4cfc-8882-610881b70cc3" />

## Features

- [x] Supports both Jellyfin and Subsonic/Navidrome backends
- [x] MPRIS
- [x] Lyrics
- [x] Replaygain (Jellyfin only)
- [x] Transcoding
- [x] Search
- [x] Playlist management
- [x] Smart Playlists 

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


### NixOS

    nix-shell -p gelly

## Using Self Signed Certificates with Jellyfin

There is currently [an issue with Flatpak](https://gitlab.com/freedesktop-sdk/freedesktop-sdk/-/issues/1905) 
that prevents sandboxed applications from reading the host's certificate store. This means if you are
hosting Jellyfin on a server with self-signed certificates which you have installed on the system
where you are trying to use the Gelly Flatpak, it will probably fail to connect.

[#15](https://github.com/Fingel/gelly/issues/15) tracks this issue. The workaround for now is to 
use an alternative installation method other than Flatpak or to connect without TLS. 
I am looking for someone to help test using alternative TLS backends for reqwest 
that might fix this issue.

## Development

Make sure you have the development libraries for the following installed:

* GTK
* Libadwaita
* Gstreamer

The name of these packages depends on your distribution, 
but will usually be something like `gstreamer-dev`. Note that Arch Linux includes development libs with the main
package, btw, so you don't need to install anything extra.

Gelly leverages [gtk-rs](https://gtk-rs.org/) for GTK bindings. 

You will also need a rust compiler installed. Gelly does *not* require any nightly 
features from Rust. 

To make things easy, also install the [just](https://github.com/casey/just) command runner. Building and 
launching a development build of Gelly should then simply be a matter of:

    just

And installing a release build:

    just release
    sudo just install
  
See the recipes in the [justfile](justfile) for other useful commands.


## Contributing
See [CONTRIBUTING.md](CONTRIBUTING.md)

## Special Thanks
@gabMus for all the great UI work and polish

@dstapp for the Subsonic backend

## Contact
I hang out on [libera.chat](https://libera.chat/) in [#gelly](irc://irc.libera.chat:6667/%23gelly)

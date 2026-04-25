<p align="center">
    <img width="150" height="150" src="https://github.com/user-attachments/assets/156e5858-a48e-4ea5-a4e4-6fbcd6644dd7" align="center" /><br />
    <br />
    <strong style="font-size: 26px;">Gelly</strong><br>
    <em>A native, lightweight music client for Jellyfin and Subsonic. Built with Rust and GTK.</em>
    🦀🐧
</p>

<img width="807" height="769" alt="Screenshot from 2026-04-25 10-11-15" src="https://github.com/user-attachments/assets/bb7338ca-bc47-41e5-ada2-3bb370060ace" />


## Features

- [x] Supports both Jellyfin and Subsonic/Navidrome backends
- [x] MPRIS
- [x] Lyrics
- [x] Replaygain
- [x] Transcoding
- [x] Search
- [x] Playlist management
- [x] Favorites 
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

## Connecting with TLS using self signed certificates

There is currently [an issue with Flatpak](https://gitlab.com/freedesktop-sdk/freedesktop-sdk/-/issues/1905) 
that prevents sandboxed applications from reading the host's certificate store. This means if you are using
a self-signed certificate on your Jellyfin/Navidrome install, Gelly will be unlikely to be able to connect
even if you have the cert installed locally.

There is a workaround: You need to make the cert file available to the flatpak sandbox and then 
set the `SSL_CERT_FILE` env var to point to it. This can be done using a tool like Flatseal. Thank you
@RodrigoPrestes for [finding this workaround](https://github.com/Fingel/gelly/issues/15#issuecomment-4195533397).

The other alternative is to use a non flatpak installation method.

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

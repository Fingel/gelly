# Gelly

Gelly is [Jellyfin Media Server](https://jellyfin.org/) client focused on music playback.

## Features

- [x] Play music!
- [x] Browse albums and artists
- [x] Simple playlist management
- [ ] MPRIS support
- [ ] Remote playlist management
- [ ] Eye candy

## Development

Gelly leverages [gtk-rs](https://gtk-rs.org/) for the UI and
[gstreamer](https://gstreamer.freedesktop.org/) for playback. Thus you will need
development libraries installed for both GTK and gstreamer to build from source.

Gelly does *not* require any nightly features from Rust.

The [justfile](justfile) contains recipes, simply running the default recipe `just` should be enough
to build and launch Gelly. Currently the only non-binary resources are glib schemas, which are placed
in ~/.local/share/glib-2.0/schemas - this will expand soon to include icons, .desktop entries, etc.


## Windows/OSX
Although there isn't anything preventing Gelly from being cross platform, it is not my focus.
I would accept reasonable PRs to enable other platforms.

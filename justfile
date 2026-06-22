prefix := "/usr"
localedir := prefix + "/share/locale"

dev: schemas
    RUST_LOG="debug,glycin=off,glycin_utils=off" GTK_DEBUG=builder cargo run

schemas:
    mkdir -p ~/.local/share/glib-2.0/schemas
    cp resources/io.m51.Gelly.gschema.xml ~/.local/share/glib-2.0/schemas/
    glib-compile-schemas ~/.local/share/glib-2.0/schemas/

pot:
    # https://crates.io/crates/xtr
    xtr src/main.rs -o po/gelly.pot
    xgettext \
        --from-code=UTF-8 \
        --keyword=tr \
        --keyword=ngettext:1,2 \
        --add-comments \
        --join-existing \
        --output=po/gelly.pot \
        --files-from=po/POTFILES.in

release:
    LOCALEDIR={{ localedir }} cargo build --release

install:
    #!/usr/bin/env bash
    set -euo pipefail
    install -Dm755 target/release/gelly {{ prefix }}/bin/gelly
    install -Dm644 resources/io.m51.Gelly.desktop {{ prefix }}/share/applications/io.m51.Gelly.desktop
    install -Dm644 resources/io.m51.Gelly.metainfo.xml {{ prefix }}/share/metainfo/io.m51.Gelly.metainfo.xml
    install -Dm644 resources/io.m51.Gelly.gschema.xml {{ prefix }}/share/glib-2.0/schemas/io.m51.Gelly.gschema.xml
    install -Dm644 resources/io.m51.Gelly.svg {{ prefix }}/share/icons/hicolor/scalable/apps/io.m51.Gelly.svg
    install -Dm644 resources/io.m51.Gelly-symbolic.svg {{ prefix }}/share/icons/hicolor/symbolic/apps/io.m51.Gelly-symbolic.svg
    glib-compile-schemas {{ prefix }}/share/glib-2.0/schemas/
    for po_file in po/*.po; do
        [[ -f "$po_file" ]] || continue
        lang=$(basename "$po_file" .po)
        install -dm755 {{ localedir }}/$lang/LC_MESSAGES
        msgfmt -o {{ localedir }}/$lang/LC_MESSAGES/gelly.mo "$po_file"
    done

uninstall:
    #!/usr/bin/env bash
    set -euo pipefail
    rm {{ prefix }}/bin/gelly
    rm {{ prefix }}/share/applications/io.m51.Gelly.desktop
    rm {{ prefix }}/share/metainfo/io.m51.Gelly.metainfo.xml
    rm {{ prefix }}/share/glib-2.0/schemas/io.m51.Gelly.gschema.xml
    rm {{ prefix }}/share/icons/hicolor/scalable/apps/io.m51.Gelly.svg
    rm {{ prefix }}/share/icons/hicolor/symbolic/apps/io.m51.Gelly-symbolic.svg
    glib-compile-schemas {{ prefix }}/share/glib-2.0/schemas/
    for lang in $(cat po/LINGUAS); do
        rm -f {{ localedir }}/$lang/LC_MESSAGES/gelly.mo
    done

dev-remote host: schemas
    #!/usr/bin/env bash
    set -euo pipefail
    echo "Building locally..."
    cargo build

    echo "Launching on remote display..."
    DEV_HOST=$(hostname -f)
    BINARY_PATH="{{ justfile_directory() }}/target/debug/gelly"
    WAYLAND_DISPLAY_VAR="${WAYLAND_DISPLAY:-wayland-0}"

    ssh {{ host }} "WAYLAND_DISPLAY=$WAYLAND_DISPLAY_VAR RUST_LOG='debug,glycin=off,glycin_utils=off' waypipe -n ssh $DEV_HOST $BINARY_PATH"

navidrome:
    podman run \
      --replace \
      --name navidrome \
      -v $MUSIC_DIR:/music:ro \
      -v ./scratch/navidrome-data:/data \
      -p 4533:4533 \
      -e ND_LOGLEVEL=info \
      deluan/navidrome:latest

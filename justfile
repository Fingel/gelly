prefix := "/usr"

dev: schemas
    RUST_LOG="debug,glycin=off,glycin_utils=off" cargo run

schemas:
    mkdir -p ~/.local/share/glib-2.0/schemas
    cp resources/io.m51.Gelly.gschema.xml ~/.local/share/glib-2.0/schemas/
    glib-compile-schemas ~/.local/share/glib-2.0/schemas/

release:
    cargo build --release

install: release
    sudo install -Dm755 target/release/gelly {{prefix}}/bin/gelly
    sudo install -Dm644 resources/io.m51.Gelly.desktop {{prefix}}/share/applications/io.m51.Gelly.desktop
    sudo install -Dm644 resources/io.m51.Gelly.metainfo.xml {{prefix}}/share/metainfo/io.m51.Gelly.metainfo.xml
    sudo install -Dm644 resources/io.m51.Gelly.gschema.xml {{prefix}}/share/glib-2.0/schemas/io.m51.Gelly.gschema.xml
    sudo install -Dm644 resources/io.m51.Gelly.svg {{prefix}}/share/icons/hicolor/scalable/apps/io.m51.Gelly.svg
    sudo install -Dm644 resources/io.m51.Gelly-symbolic.svg {{prefix}}/share/icons/hicolor/symbolic/apps/io.m51.Gelly-symbolic.svg
    sudo glib-compile-schemas {{prefix}}/share/glib-2.0/schemas/

uninstall:
    sudo rm {{prefix}}/bin/gelly
    sudo rm {{prefix}}/share/applications/io.m51.Gelly.desktop
    sudo rm {{prefix}}/share/metainfo/io.m51.Gelly.metainfo.xml
    sudo rm {{prefix}}/share/glib-2.0/schemas/io.m51.Gelly.gschema.xml
    sudo rm {{prefix}}/share/icons/hicolor/scalable/apps/io.m51.Gelly.svg
    sudo rm {{prefix}}/share/icons/hicolor/symbolic/apps/io.m51.Gelly-symbolic.svg
    sudo glib-compile-schemas {{prefix}}/share/glib-2.0/schemas/

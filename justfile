prefix := "/usr"

dev: schemas
    RUST_LOG="debug,glycin=off,glycin_utils=off" cargo run

schemas:
    mkdir -p ~/.local/share/glib-2.0/schemas
    cp resources/io.m51.Gelly.gschema.xml ~/.local/share/glib-2.0/schemas/
    glib-compile-schemas ~/.local/share/glib-2.0/schemas/

release:
    cargo build --release

install:
    install -Dm755 target/release/gelly {{prefix}}/bin/gelly
    install -Dm644 resources/io.m51.Gelly.desktop {{prefix}}/share/applications/io.m51.Gelly.desktop
    install -Dm644 resources/io.m51.Gelly.metainfo.xml {{prefix}}/share/metainfo/io.m51.Gelly.metainfo.xml
    install -Dm644 resources/io.m51.Gelly.gschema.xml {{prefix}}/share/glib-2.0/schemas/io.m51.Gelly.gschema.xml
    install -Dm644 resources/io.m51.Gelly.svg {{prefix}}/share/icons/hicolor/scalable/apps/io.m51.Gelly.svg
    install -Dm644 resources/io.m51.Gelly-symbolic.svg {{prefix}}/share/icons/hicolor/symbolic/apps/io.m51.Gelly-symbolic.svg
    glib-compile-schemas {{prefix}}/share/glib-2.0/schemas/

uninstall:
    rm {{prefix}}/bin/gelly
    rm {{prefix}}/share/applications/io.m51.Gelly.desktop
    rm {{prefix}}/share/metainfo/io.m51.Gelly.metainfo.xml
    rm {{prefix}}/share/glib-2.0/schemas/io.m51.Gelly.gschema.xml
    rm {{prefix}}/share/icons/hicolor/scalable/apps/io.m51.Gelly.svg
    rm {{prefix}}/share/icons/hicolor/symbolic/apps/io.m51.Gelly-symbolic.svg
    glib-compile-schemas {{prefix}}/share/glib-2.0/schemas/

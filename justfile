dev: schemas
    RUST_LOG="debug,glycin=off,glycin_utils=off" cargo run

schemas:
    mkdir -p ~/.local/share/glib-2.0/schemas
    cp resources/io.m51.Gelly.gschema.xml ~/.local/share/glib-2.0/schemas/
    glib-compile-schemas ~/.local/share/glib-2.0/schemas/

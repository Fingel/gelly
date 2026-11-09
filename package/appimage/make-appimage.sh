#!/usr/bin/env bash

set -e

export ID=io.m51.Gelly
export NAME=Gelly
export BIN=gelly
export PROFILE="${PROFILE:=release}"
export APPIMAGE_APPDIR="target/$NAME.AppDir"
export APPIMAGE_DIST="target/appimage"
export ARCH=$(uname -m)
export GSTREAMER_VERSION=1.0
export GSTREAMER_PLUGINS_DIR="$(pkg-config --variable=pluginsdir gstreamer-1.0)"
export GSTREAMER_HELPERS_DIR="$(pkg-config --variable=pluginscannerdir gstreamer-1.0)"

wget https://github.com/linuxdeploy/linuxdeploy/releases/download/continuous/linuxdeploy-$ARCH.AppImage -O /tmp/linuxdeploy.appimage
chmod +x /tmp/linuxdeploy.appimage
wget https://raw.githubusercontent.com/linuxdeploy/linuxdeploy-plugin-gstreamer/refs/heads/master/linuxdeploy-plugin-gstreamer.sh -O /tmp/linuxdeploy-plugin-gstreamer.sh
chmod +x /tmp/linuxdeploy-plugin-gstreamer.sh
mkdir -p $APPIMAGE_APPDIR
mkdir -p $APPIMAGE_DIST
install -Dm 0755 target/$PROFILE/$BIN "$APPIMAGE_APPDIR/usr/bin/$BIN"
install -Dm 0755 package/appimage/AppRun "$APPIMAGE_APPDIR/AppRun"
install -Dm 0644 resources/$ID.desktop "$APPIMAGE_APPDIR/usr/share/applications/$ID.desktop"
install -Dm 0644 resources/$ID.metainfo.xml "$APPIMAGE_APPDIR/usr/share/metainfo/$ID.metainfo.xml"
install -Dm 0644 resources/$ID.gschema.xml "$APPIMAGE_APPDIR/usr/share/glib-2.0/schemas/$ID.gschema.xml"
install -Dm 0644 resources/$ID.svg "$APPIMAGE_APPDIR/usr/share/icons/hicolor/scalable/apps/$ID.svg"
install -Dm 0644 resources/$ID-symbolic.svg "$APPIMAGE_APPDIR/usr/share/icons/hicolor/symbolic/apps/$ID-symbolic.svg"
glib-compile-schemas $APPIMAGE_APPDIR/usr/share/glib-2.0/schemas/
for po_file in po/*.po; do
    lang=$(basename "$po_file" .po)
    install -dm755 "$APPIMAGE_APPDIR/usr/share/locale/$lang/LC_MESSAGES"
    msgfmt -o "$APPIMAGE_APPDIR/usr/share/locale/$lang/LC_MESSAGES/gelly.mo" "$po_file"
done
/tmp/linuxdeploy.appimage --appimage-extract-and-run \
    --appdir $APPIMAGE_APPDIR \
    --executable target/$PROFILE/$BIN \
    --desktop-file resources/$ID.desktop \
    --plugin gstreamer \
    --exclude-library='libglib-2.0.so*' \
    --exclude-library='libgobject-2.0.so*' \
    --exclude-library='libgio-2.0.so*' \
    --exclude-library='libgmodule-2.0.so*' \
    --exclude-library='libgthread-2.0.so*' \
    --exclude-library='libxkbcommon.so*' \
    --exclude-library='libxkbcommon-x11.so*' \
    --exclude-library='libX11.so*' \
    --exclude-library='libxcb.so*' \
    --output appimage
mv Gelly-$ARCH.AppImage "$APPIMAGE_DIST/gelly-$PROFILE-$ARCH.appimage"
rm -rf $APPIMAGE_APPDIR

{ lib
, fetchFromGitHub
, rustPlatform
, pkg-config
, gtk4
, libadwaita
, glib
, wrapGAppsHook4
, dbus
, openssl
, gst_all_1
, libseccomp
}:

rustPlatform.buildRustPackage rec {
	pname = "gelly";
	version = "0.14.0";

	src = fetchFromGitHub {
		owner  = "Fingel";
		repo   = pname;
		rev    = "v${version}";
		hash   = "sha256-7EmRC8qFN0q9O8FsQiSqYqEfvCg7yKKOxLca098867A=";
	};

	cargoHash = "sha256-INxtgEg1a8PK0BofySwB2OsLFlVZaiz9nbXGNKB+icE=";

	nativeBuildInputs = [
		pkg-config
		wrapGAppsHook4
		glib
	];

	buildInputs = [
		gtk4
		libadwaita
		glib
		dbus
		openssl
		libseccomp
	] ++ (with gst_all_1; [
		gstreamer
		gst-plugins-base
		gst-plugins-good
		gst-plugins-bad
		gst-plugins-ugly
		gst-libav
	]);

	preFixup = ''
		glib-compile-schemas $out/share/gsettings-schemas/${pname}-${version}/glib-2.0/schemas
	'';

	# Install desktop file, icons and schemas
	postInstall = ''
		install -Dm644 resources/io.m51.Gelly.desktop \
		$out/share/applications/io.m51.Gelly.desktop

		install -Dm644 resources/io.m51.Gelly.metainfo.xml \
		$out/share/metainfo/io.m51.Gelly.metainfo.xml

		install -Dm644 resources/io.m51.Gelly.gschema.xml \
		$out/share/glib-2.0/schemas/io.m51.Gelly.gschema.xml

		install -Dm644 resources/io.m51.Gelly.svg \
		$out/share/icons/hicolor/scalable/apps/io.m51.Gelly.svg

		install -Dm644 resources/io.m51.Gelly-symbolic.svg \
		$out/share/icons/hicolor/symbolic/apps/io.m51.Gelly-symbolic.svg
	'';

	meta = with lib; {
		description = "A Jellyfin media-server client focused on music";
		homepage      = "https://github.com/Fingel/gelly";
		license       = licenses.gpl3Plus;
		maintainers   = with maintainers; [ ];
		platforms     = platforms.x86_64 ++ platforms.aarch64;
	};
}

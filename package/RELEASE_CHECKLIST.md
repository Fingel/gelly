- [ ] Update Cargo.toml
- [ ] Update resources/io.m51.Gelly.metainfo.xml
- [ ] Run the metainfo linter: `flatpak run --command=flatpak-builder-lint org.flatpak.Builder appstream resources/io.m51.Gelly.metainfo.xml`
- [ ] Git tag
- [ ] Github release

Arch/PKGBUILD:

- [ ] Update pkgver
- [ ] Update sha512sum of release tarball
- [ ] Copy PKGBUILD to AUR repo
- [ ] Run `makepkg --printsrcinfo > .SRCINFO` to update .SRCINFO
- [ ] Commit and push

Flatpak:
- [ ] Update sources in io.m51.Gelly.yml
- [ ] Update cargo-sources.json `uv run scratch/flatpak-cargo-generator.py Cargo.lock -o package/flatpak/cargo-sources.json`
- [ ] Run the flatpak linter: `flatpak run --command=flatpak-builder-lint org.flatpak.Builder manifest package/flatpak/io.m51.Gelly.yml`
- [ ] Build the flatpak: `flatpak run --command=flathub-build org.flatpak.Builder package/flatpak/io.m51.Gelly.yml`
- [ ] Install the flatpak: `flatpak install --user -y ./repo io.m51.Gelly`
- [ ] Lint the repo: `flatpak run --command=flatpak-builder-lint org.flatpak.Builder repo repo`
- [ ] Open PR for updated manifest

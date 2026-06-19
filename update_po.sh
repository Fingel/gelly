#!/usr/bin/env bash

for po_file in po/*.po; do
 msgmerge --no-fuzzy-matching --update --backup=none "$po_file" po/gelly.pot
done

echo "Done."

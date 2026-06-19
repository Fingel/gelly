#!/usr/bin/env bash

# to get xtr run "cargo install xtr"

xtr src/main.rs -o po/gelly.pot

grep resources po/POTFILES.in > po/POTFILES2.in

xgettext --join-existing --output=po/gelly.pot --files-from=po/POTFILES2.in --add-comments

rm -f po/POTFILES2.in

for po_file in po/*.po; do
 msgmerge --no-fuzzy-matching --update --backup=none "$po_file" po/gelly.pot
done

echo "Done."

#!/bin/sh
set -eu

status=0

for file in $(find crates/*/src -name '*.rs' | sort); do
    folder=$(basename "$(dirname "$file")")
    [ "$folder" = src ] && continue

    name=$(basename "$file" .rs)
    [ "$name" = mod ] && continue

    single=${folder%s}
    case $folder in
        *ies) single=${folder%ies}y ;;
    esac

    case $name in
        "$folder" | "$single" | "$folder"_* | "$single"_*)
            echo "$file: a file in $folder/ repeats the folder's name"
            status=1
            ;;
    esac
done

exit $status

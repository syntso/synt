#!/bin/sh
set -eu

status=0

fail() {
    echo "$1"
    status=1
}

single_of() {
    case $1 in
        *ies) echo "${1%ies}y" ;;
        *s) echo "${1%s}" ;;
        *) echo "$1" ;;
    esac
}

roots=$(find crates -mindepth 2 -maxdepth 2 -type d \( -name src -o -name tests -o -name benches -o -name examples \) | sort)

for root in $roots; do
    for file in $(find "$root" -name mod.rs | sort); do
        fail "$file: no mod.rs files; put the module in the file beside its folder"
    done

    for path in $(find "$root" -mindepth 2 \( -name '*.rs' -o -type d \) | sort); do
        parent=$(basename "$(dirname "$path")")
        name=$(basename "$path" .rs)
        single=$(single_of "$parent")

        case $name in
            "$single"*) fail "$path: starts with its folder's name $parent" ;;
        esac
    done

    for folder in $(find "$root" -type d | sort); do
        prefixes=$(
            find "$folder" -mindepth 1 -maxdepth 1 \( -name '*.rs' -o -type d \) -exec basename {} .rs \; |
                sort -u |
                awk -F_ '
                    { names[$0] = 1; if (NF > 1) { count[$1]++ } }
                    END {
                        for (prefix in count) {
                            if (count[prefix] > 1 || (prefix in names)) { print prefix }
                        }
                    }
                '
        )

        for prefix in $prefixes; do
            fail "$folder: several names start with ${prefix}_; make $prefix a folder"
        done
    done
done

exit $status

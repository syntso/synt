#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "$0")/.." && pwd)"
out="$root/target/swift"
work="$out/work"
package="$out/SyntClient"
targets=(aarch64-apple-darwin x86_64-apple-darwin)
library=libsynt_swift.a

export MACOSX_DEPLOYMENT_TARGET="${MACOSX_DEPLOYMENT_TARGET:-14.0}"

cd "$root"
rm -rf "$out"
mkdir -p "$work/headers/SyntClientFFI" "$work/generated" "$package/Sources/SyntClient"

for target in "${targets[@]}"; do
    cargo build --locked --release -p synt-swift --lib --target "$target"
done

lipo -create \
    "target/aarch64-apple-darwin/release/$library" \
    "target/x86_64-apple-darwin/release/$library" \
    -output "$work/$library"

cargo run --locked -p synt-swift --features bindgen --bin uniffi-bindgen -- \
    generate --library "target/aarch64-apple-darwin/release/$library" \
    --language swift --out-dir "$work/generated"

cp "$work/generated/SyntClientFFI.h" "$work/headers/SyntClientFFI/"
cp "$work/generated/SyntClientFFI.modulemap" "$work/headers/SyntClientFFI/module.modulemap"

xcodebuild -create-xcframework \
    -library "$work/$library" \
    -headers "$work/headers" \
    -output "$out/SyntClient.xcframework"

cp "$work/generated/SyntClient.swift" "$package/Sources/SyntClient/"
cp -R "$out/SyntClient.xcframework" "$package/"

cat > "$package/Package.swift" <<'SWIFT'
// swift-tools-version:5.9
import PackageDescription

let package = Package(
    name: "SyntClient",
    platforms: [.macOS(.v14)],
    products: [
        .library(name: "SyntClient", targets: ["SyntClient"]),
    ],
    targets: [
        .binaryTarget(name: "SyntClientFFI", path: "SyntClient.xcframework"),
        .target(name: "SyntClient", dependencies: ["SyntClientFFI"]),
    ]
)
SWIFT

(cd "$out" && ditto -c -k --sequesterRsrc --keepParent SyntClient.xcframework SyntClient.xcframework.zip)
(cd "$package" && ditto -c -k --keepParent Sources/SyntClient "$out/SyntClient-swift.zip")

checksum="$(swift package compute-checksum "$out/SyntClient.xcframework.zip")"
echo "$checksum" > "$out/SyntClient.xcframework.zip.checksum"

rm -rf "$work"

echo "xcframework: $out/SyntClient.xcframework.zip ($checksum)"
echo "package:     $package"

# Synt releases

Builds of [Synt](https://synt.so) for macOS, Windows and Linux. The source of Synt is closed; the parts others build on are open: [mcp](https://github.com/syntso/mcp).

Download the archive for your system from [Releases](https://github.com/syntso/releases/releases) and check it against `SHA256SUMS`:

```bash
shasum -a 256 -c SHA256SUMS --ignore-missing
```

| Archive | System |
|---|---|
| `synt-core-macos-arm64.tar.gz` | macOS on Apple silicon |
| `synt-core-macos-x64.tar.gz` | macOS on Intel |
| `synt-core-linux-x64.tar.gz` | Linux x86-64 |
| `synt-core-linux-arm64.tar.gz` | Linux ARM64 |
| `synt-core-windows-x64.zip` | Windows x86-64 |

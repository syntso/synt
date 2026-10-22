<img src="docs/images/banner.webp" alt="Synt" width="100%" />

<p align="center">
  <a href="https://github.com/syntso/synt/releases/latest"><img src="https://img.shields.io/github/v/release/syntso/synt?style=flat-square&color=555" alt="Release" /></a>
  <img src="https://img.shields.io/badge/platforms-macOS%20%C2%B7%20Windows%20%C2%B7%20Linux-555?style=flat-square" alt="Platforms" />
  <a href="https://github.com/syntso/mcp"><img src="https://img.shields.io/badge/AI%20agents-MCP-555?style=flat-square" alt="MCP" /></a>
</p>

<p align="center">
  <b>A workspace for servers, Kubernetes clusters and databases.</b><br />
  You and your AI agents use the same actions, and nothing touches a host until you allow it.
</p>

<p align="center">
  <a href="https://synt.so">Website</a>
  &nbsp;·&nbsp;
  <a href="#install">Install</a>
  &nbsp;·&nbsp;
  <a href="#ai-agents">AI agents</a>
  &nbsp;·&nbsp;
  <a href="https://github.com/syntso/synt/releases">Releases</a>
</p>

<br />

This repository is where Synt is published. The builds of every version are under [Releases](https://github.com/syntso/synt/releases).

<br />

## What's inside

<table>
  <tr><td><b>SSH</b></td><td>ProxyJump, terminal sessions that track each command, resumable file transfers, tunnels</td></tr>
  <tr><td><b>Kubernetes</b></td><td>Clusters reached directly or through an SSH host; apply with a server-side diff, logs, exec, port-forward</td></tr>
  <tr><td><b>Databases</b></td><td>PostgreSQL, MySQL, SQLite and Redis; every statement is judged by what it does, reads run read-only</td></tr>
  <tr><td><b>Runbooks</b></td><td>Steps with checks, retries and rollback; drafted from your terminal, exported as a shell script</td></tr>
  <tr><td><b>AI agents</b></td><td>An <a href="https://modelcontextprotocol.io">MCP</a> way in with the same actions and approvals as you</td></tr>
  <tr><td><b>Extensions</b></td><td>WebAssembly in a sandbox: actions, native views, web panels, documents</td></tr>
  <tr><td><b>Sync</b></td><td>Projects shared between devices and teammates, end-to-end encrypted</td></tr>
  <tr><td><b>Journal</b></td><td>Every call recorded in a hash chain that can be verified but not rewritten</td></tr>
</table>

<br />

## How it works

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="docs/images/how-it-works-dark.svg" />
  <img src="docs/images/how-it-works-light.svg" alt="The app and AI agents talk to the Synt core; its broker asks before acting, writes to the journal and runs actions on SSH hosts, Kubernetes and databases" width="100%" />
</picture>

<br />

- Before an action touches a host, Synt asks you: once, until restart, or always. Privileged commands from an agent are asked every time.
- SSH keys and passwords stay in the system keychain. Agents and extensions refer to them by name.
- Requests, approvals and outcomes go to the journal, without the data itself.

<br />

## Install

Each archive holds two programs: `synt-core`, the core, and `synt-mcp`, the bridge for AI agents. Keep them in the same folder.

| System | Archive |
|---|---|
| macOS, Apple silicon | `synt-macos-arm64.tar.gz` |
| macOS, Intel | `synt-macos-x64.tar.gz` |
| Linux, x86-64 | `synt-linux-x64.tar.gz` |
| Linux, ARM64 | `synt-linux-arm64.tar.gz` |
| Windows, x86-64 | `synt-windows-x64.zip` |

### macOS and Linux

```bash
curl -fsSLO https://github.com/syntso/synt/releases/latest/download/synt-macos-arm64.tar.gz
curl -fsSLO https://github.com/syntso/synt/releases/latest/download/SHA256SUMS
shasum -a 256 -c SHA256SUMS --ignore-missing
mkdir -p ~/.local/bin && tar -xzf synt-macos-arm64.tar.gz -C ~/.local/bin
```

Use the archive name for your system in the first line. On Linux, `sha256sum -c SHA256SUMS --ignore-missing` does the same check. Make sure `~/.local/bin` is on your `PATH`.

The macOS builds are not signed yet. Until they are, start the core with `--allow-login-keychain`: it then keeps its journal key in your login keychain, which other programs of your user can read.

### Windows

```powershell
Invoke-WebRequest https://github.com/syntso/synt/releases/latest/download/synt-windows-x64.zip -OutFile synt.zip
Expand-Archive synt.zip -DestinationPath "$env:LOCALAPPDATA\Synt"
```

<br />

## AI agents

[Synt MCP](https://github.com/syntso/mcp) gives Claude Code, Claude Desktop, Cursor, Codex, VS Code and any other MCP client the actions of Synt as tools. It starts the core from its own folder when the core is not running:

```bash
claude mcp add synt -- synt-mcp
```

On macOS, add `--allow-login-keychain` at the end until the builds are signed. Setup for every client is in [its README](https://github.com/syntso/mcp#connect-your-client).

<br />

## Build on Synt

| Repository | What it is | License |
|---|---|---|
| [sdk](https://github.com/syntso/sdk) | Write extensions in WebAssembly: the extension API and the Rust SDK | Apache-2.0 |
| [extensions](https://github.com/syntso/extensions) | The registry of extensions, built and signed by Synt | Apache-2.0 |
| [protocol](https://github.com/syntso/protocol) | The protocol of the core and a client for it | Apache-2.0 |
| [mcp](https://github.com/syntso/mcp) | The bridge for AI agents | Apache-2.0 |
| [ssh](https://github.com/syntso/ssh) | The SSH library inside Synt | AGPL-3.0 or commercial |

<br />

## License

Synt itself is closed source. The parts others build on are open, as listed above. For a commercial license of the SSH library, write to license@synt.so.

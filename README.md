<div align="center">

<img src="docs/images/logo.png" alt="Synt" width="96" />

# Synt

**Your servers, for you and your AI agent. The keys stay with you.**

</div>

---

A macOS app for SSH, terminals and files, with a way in for AI agents over [MCP](https://modelcontextprotocol.io).

- **One door.** You and your agent go through the same core, so the agent can never do more than you.
- **You approve.** Touching a host asks first: once, for the session, or always. `sudo` asks every time.
- **Everything is recorded.** A journal that can be read but never rewritten.
- **Keys never leave.** They stay in Keychain or ssh-agent. Nothing else ever sees them.

## Status

Early, nothing to install yet. macOS first.

- [x] Journal
- [ ] Core and protocol
- [ ] SSH with jump hosts
- [ ] Terminal
- [ ] Approvals
- [ ] macOS app
- [ ] MCP

## Build

```bash
git clone https://github.com/syntso/synt.git && cd synt
git config core.hooksPath .githooks
cargo test --workspace
```

## License

[FSL-1.1-ALv2](LICENSE.md): use and change it freely, just don't ship a competing product with it. Each version becomes Apache 2.0 two years after release.

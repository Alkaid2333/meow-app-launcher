# Meow App Launcher (meowal)

A cross-platform, high-performance app launcher (like macOS Spotlight) for Windows and Linux, built with **Rust + GPUI**.

**Phase 1 (current): Windows basic feature loop — working.** Hotkey summon → fuzzy search (EN/CN/Pinyin) → keyboard navigation → launch.

See [README.zh.md](README.zh.md) for full docs (Chinese).

## Quick Start

```bash
cargo run                    # dev mode
MEOWAL_VERBOSE=1 cargo run   # with debug logs
cargo test                   # 15 unit tests
```

Default hotkey: `Ctrl+Alt+Space`. Data lives in `./.datas` (config/apps JSON + icon PNG snapshots + rolling log).

## Architecture

Layered, inspired by WinIsland: `app/` (global state) · `views/` (launcher) · `ui/` (theme & items) · `platform/` (the only `cfg(target_os)` place) · `animation/` (pure-Rust springs) · `search/` (fuzzy + pinyin) · `apps/` (registry/scanner/icons) · `utils/` (logging).

## License

MIT

# Blitzblank

Blitzblank is an experimental macOS cleanup app written in Rust with egui. It was created using **Craft Anything**: I pointed the tool at my installed CleanMyMac app and asked it to build a copy. It took only minutes and one or two clicks. This is an experiment in what the tool can generate, not an affiliated or endorsed CleanMyMac product.

## Use at your own risk

**This software is provided without any warranty.** It can move files to the Trash, permanently empty the Trash, stop processes and run maintenance commands. Review every selection carefully and keep backups. Scans may be incomplete when macOS denies access. Its protection checks are simple heuristics, not malware detection. Moving files to the Trash does not necessarily free disk space, even if the app reports it as freed.

This is an early generated prototype, not a feature-complete or verified replacement for CleanMyMac. Some cleanup candidates and application data are selected by default. There are currently no automated tests.

## Build and run

Requires macOS and a Rust toolchain:

```sh
cargo build --release
./target/release/blitzblank
```

The interface is currently in German. The source includes cleanup, background service inspection, performance tools, application removal, duplicate and large file discovery, and a storage map.

## License

MIT. See [LICENSE](LICENSE).

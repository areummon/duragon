# End-to-end checks against the real desktop app

These drive the real Duragon window (real Rust backend, real `claude` CLI) through WebDriver.

One-time setup, inside `nix develop`:

```sh
cargo install tauri-driver --locked --root .tools
```

Run (each in its own terminal, inside `nix develop`):

```sh
npm run dev                                   # the debug build loads the UI from vite
cargo build -p duragon
XDG_DATA_HOME=/tmp/duragon-e2e .tools/bin/tauri-driver \
  --native-driver "$(dirname "$(pkg-config --variable=libdir webkit2gtk-4.1)")/bin/WebKitWebDriver"
node scripts/e2e/read-along.mjs "$PWD/target/debug/duragon" /tmp
```

`XDG_DATA_HOME` keeps the test database away from your real notes. `read-along.mjs` makes two real model calls (one read, one answer plus a drawing) and saves screenshots to the output folder.

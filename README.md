# Artoria

Rust + GPUI で作るデータベース構築ツール。

## 目次

- [ロードマップ](./docs/roadmap.md)
- [ビルド・デバッグ・リリース手順](./docs/build-and-debug.md)

## 実行

```powershell
cd D:\Workspace\Artoria
cargo run -p app
```

## ビルド

```powershell
cd D:\Workspace\Artoria
cargo build -p app
cargo build --release -p app
```

## CI

```powershell
cd D:\Workspace\Artoria
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --all-targets
```

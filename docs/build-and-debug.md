# ビルド・デバッグ・リリース手順

このリポジトリは Cargo workspace で構成されており、アプリ本体は `app` パッケージとして実行します。

## ファイル・フォルダ構成

```text
Artoria/
├── .github/
│   └── workflows/
│       └── ci.yml              # CI: fmt / clippy / test
├── app/
│   ├── Cargo.toml             # app パッケージ定義
│   └── src/
│       └── main.rs            # GPUI の起動コード
├── core/
│   ├── Cargo.toml             # core パッケージ定義
│   └── src/
│       └── lib.rs             # スキーマモデルとDDL生成の基盤
├── db/
│   ├── Cargo.toml             # db パッケージ定義
│   └── src/
│       └── lib.rs             # SQL Server 接続とDDL実行の基盤
├── docs/
│   ├── roadmap.md             # 機能ロードマップ
│   └── build-and-debug.md     # 本ドキュメント
├── .gitignore
├── Cargo.lock
├── Cargo.toml                 # workspace の定義
└── README.md (必要に応じて追加)
```

## ワークスペース設定のポイント

- ルートの `Cargo.toml` では `workspace.members` に `app`, `core`, `db` を登録しています
- 実行対象のアプリ本体は `app` パッケージです
- 旧来の `src/main.rs` ルートバイナリや `cargo run -p Artoria` は現状の構成では使いません

## ビルド

### 1. 依存の確認とコンパイル

```powershell
cd D:\Workspace\Artoria
cargo check -p app
```

### 2. 通常ビルド

```powershell
cd D:\Workspace\Artoria
cargo build -p app
```

### 3. 全体の品質チェック

```powershell
cd D:\Workspace\Artoria
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --all-targets
```

## デバッグ実行

### 1. 通常の実行

```powershell
cd D:\Workspace\Artoria
cargo run -p app
```

これで `app` パッケージの GPUI アプリが起動します。

### 2. VS Code でのデバッグ

この環境では、VS Code のデバッグ設定は「Rust 用のデバッガー拡張」が入っていないと使えません。 そのため、現時点では `launch.json` を作りすぎるよりも、タスク経由で `cargo run -p app` を実行し、必要に応じてブラレークポイントを貼る運用が安定です。

以下のタスクが利用可能です。

```powershell
cargo run -p app
cargo run --release -p app
```

または VS Code のタスク一覧から `cargo: run app` を選んで実行します。

注: 「CodeLLDB」や Rust のデバッグアダプターが未導入の環境では、`launch.json` のデバッグ設定自体が認識されません。 その場合は、まずデバッガー拡張を入れてから、`launch.json` を有効化してください。

## リリース実行

### 1. リリースビルド

```powershell
cd D:\Workspace\Artoria
cargo build --release -p app
```

### 2. リリース実行

```powershell
cd D:\Workspace\Artoria
cargo run --release -p app
```

### 3. 生成済みバイナリを直接実行

```powershell
cd D:\Workspace\Artoria
.\target\release\app.exe
```

## よくあるミス

- `cargo run -p app` を使うべきところで `cargo run -p Artoria` を使う
- ルートの `src/main.rs` を残したまま、別のバイナリを実行しようとする
- 実行対象のパッケージ名を workspace 内の `app` ではなく旧名と勘違いする

正しい実行対象は `app` です。

## 参考

- [roadmap.md](./roadmap.md)
- [Cargo.toml](../Cargo.toml)

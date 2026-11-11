# Artoria Phase 1 手順書: スキーマモデルとDDL生成(artoria-core)

ゴール: 画面もDB接続もなしで、「スキーマモデル → SQL ServerのDDL」が完成している状態にする。

## 進捗表

| Step | 内容 | 完了 |
|---|---|---|
| 0 | ワークスペースとクレートの作成 | [ ] |
| 1 | モデルの型定義 | [ ] |
| 2 | 識別子のクォートと型のSQL化 | [ ] |
| 3 | `CREATE TABLE` の生成 | [ ] |
| 4 | 外部キー(`ALTER TABLE`)の生成 | [ ] |
| 5 | インデックスの生成 | [ ] |
| 6 | スキーマ全体のDDL生成 | [ ] |
| 7 | バリデーション | [ ] |
| 8 | JSON保存の往復テスト | [ ] |
| 9 | 仕上げ | [ ] |

**進め方のルール**
- 1ステップ = 1コミット。ステップの最後に必ず `cargo test` を通す
- 迷ったら、各ステップの「判断メモ」に書いてある方を選んでおく。後から変えられる
- 全部のステップを一度に考えない。今のステップだけ見る

---

## Step 0: ワークスペースとクレートの作成

**やること**

```bash
mkdir artoria && cd artoria
git init
mkdir crates
cargo new --lib crates/artoria-core
```

ルートの `Cargo.toml` を作る。

```toml
[workspace]
resolver = "2"
members = ["crates/artoria-core"]
```

`crates/artoria-core/Cargo.toml` に依存を足す。

```toml
[dependencies]
serde = { version = "1", features = ["derive"] }
serde_json = "1"
thiserror = "2"
```

**説明**

- **クレート名を `core` にしない。** `core` はRust標準ライブラリの名前で、衝突して意味不明なエラーが出る。製品名に合わせて `artoria-core`(コード内では `artoria_core`)にする。以降のクレートも `artoria-db` / `artoria-app` に揃える
- 今は `artoria-core` だけをメンバーにする。`artoria-db` と `artoria-app` は必要になったら足す
- `thiserror` はPhase 1後半のバリデーションエラー用

**確認**: `cargo test` が通る(テストは0件でよい)。

---

## Step 1: モデルの型定義

**やること**: `src/model.rs` を作り、`lib.rs` から `pub mod model;` で公開する。次の型を定義する。

| 型 | 種類 | 持たせるもの |
|---|---|---|
| `SqlType` | enum | Int / BigInt / Bit / DateTime2 / UniqueIdentifier / Decimal(精度, 小数桁) / NVarChar(長さ。MAXは `None`) |
| `Column` | struct | name, ty, nullable, identity, default |
| `Table` | struct | schema, name, columns, primary_key(カラム名の配列), foreign_keys, indexes |
| `ForeignKey` | struct | name, columns, ref_schema, ref_table, ref_columns |
| `Index` | struct | name, columns, unique |
| `Schema` | struct | tables |

全部に `#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]` を付ける。`Schema` には `Default` も付ける。

**説明**

- `PartialEq` はPhase 6(差分計算)で必要になる。今付けておけば後で型を触らずに済む
- `Serialize` / `Deserialize` はPhase 3(JSON保存)用。これも今付けておく
- 主キーは `Table` 側にカラム名の配列として持たせる。カラム側に「PKフラグ」を持たせない理由は、複合キーのとき順序が必要になるため

**判断メモ**
- `default` は **生のSQL文字列**(`Option<String>`)にする。例: `"GETDATE()"`, `"0"`, `"N'abc'"`。型付きの値にするのは複雑なので、今はやらない
- 制約名(PK・FK・インデックス)のうち、PKは **名前を持たせず自動命名**(`PK_テーブル名`)にする。FKとインデックスは名前を持たせる(上の表の通り)
- テーブル・カラムの作成順は `Vec` の並び順を保つ(カラムの並びはそのままDDLの並びになるため)

**確認**: 型を組み立てるだけのテストを1つ書く(コンパイルが通り、`clone` して `==` で比較できる)。

---

## Step 2: 識別子のクォートと型のSQL化

**やること**: `src/ddl.rs` を作り、2つの関数を書く(外には公開しなくてよい)。

- `quote(name: &str) -> String`
- `type_sql(ty: &SqlType) -> String`

**ルール**

- `quote`: `[` と `]` で囲む。名前の中に `]` があれば `]]` に置き換える
- `type_sql`: 次の表の通り

| `SqlType` | 出力 |
|---|---|
| Int | `INT` |
| BigInt | `BIGINT` |
| Bit | `BIT` |
| DateTime2 | `DATETIME2` |
| UniqueIdentifier | `UNIQUEIDENTIFIER` |
| Decimal(18, 2) | `DECIMAL(18,2)` |
| NVarChar(Some(100)) | `NVARCHAR(100)` |
| NVarChar(None) | `NVARCHAR(MAX)` |

**テストで確認すること**

- `quote("Users")` → `[Users]`
- `quote("a]b")` → `[a]]b]`
- 上の表の全パターン

---

## Step 3: `CREATE TABLE` の生成

**やること**: `create_table_sql(&Table) -> String` を書く。

**期待する出力(これをテストの正解にする)**

```sql
CREATE TABLE [dbo].[Users] (
    [Id] INT IDENTITY(1,1) NOT NULL,
    [Name] NVARCHAR(100) NOT NULL,
    [Email] NVARCHAR(255) NULL,
    [CreatedAt] DATETIME2 NOT NULL DEFAULT GETDATE(),
    CONSTRAINT [PK_Users] PRIMARY KEY ([Id])
);
```

**書く順序の考え方**

1. 各カラムを1行にする: `名前 型 [IDENTITY(1,1)] NULL/NOT NULL [DEFAULT 式]`
2. 主キーがあれば、最後に `CONSTRAINT` の行を足す
3. 全部を `,\n` でつなぎ、`CREATE TABLE [スキーマ].[テーブル] (\n ... \n);` で包む

**注意**
- 行の頭のインデントは **スペース4つ** に固定する(テストの文字列比較が崩れないように)
- 最後の要素の後ろにカンマを付けない

**テストで確認すること**

- 上の例そのまま
- 主キーなしのテーブル(`CONSTRAINT` 行が出ない)
- 複合主キー(`PRIMARY KEY ([A], [B])`)
- テーブル名に `]` を含む場合

---

## Step 4: 外部キー(`ALTER TABLE`)の生成

**やること**: `add_foreign_key_sql(&Table, &ForeignKey) -> String` を書く。

**期待する出力**

```sql
ALTER TABLE [dbo].[Orders] ADD CONSTRAINT [FK_Orders_Users] FOREIGN KEY ([UserId]) REFERENCES [dbo].[Users] ([Id]);
```

**説明**: FKを `CREATE TABLE` の中に入れず、後から `ALTER TABLE` で付ける。そうすると、テーブルの作成順(依存関係の並べ替え)を考えなくて済み、循環参照があっても動く。

**テストで確認すること**: 単一カラムのFK、複合カラムのFK(`([A], [B]) REFERENCES ... ([X], [Y])`)。

---

## Step 5: インデックスの生成

**やること**: `create_index_sql(&Table, &Index) -> String` を書く。

**期待する出力**

```sql
CREATE UNIQUE INDEX [IX_Users_Email] ON [dbo].[Users] ([Email]);
CREATE INDEX [IX_Orders_UserId] ON [dbo].[Orders] ([UserId], [CreatedAt]);
```

**判断メモ**: クラスター化・非クラスター化の指定はまだ入れない(指定なしなら非クラスター化になる)。必要になったらPhase 7で足す。

**テストで確認すること**: UNIQUEあり・なし、複数カラム。

---

## Step 6: スキーマ全体のDDL生成

**やること**: `generate_ddl(&Schema) -> Vec<String>` を書く。

**出力の順序(重要)**

1. 全テーブルの `CREATE TABLE`
2. 全テーブルの外部キー(`ALTER TABLE`)
3. 全テーブルのインデックス

**説明**

- 戻り値は **1つの長い文字列ではなく、文のリスト**(`Vec<String>`)にする。Phase 4で1文ずつ実行するときと、画面で文ごとに表示するときに、そのまま使えるため
- 画面のプレビュー用に連結したいときは `join("\n\n")` すればよい
- この時点ではバリデーションをしない(Step 7で足す)

**テストで確認すること**: 2テーブル + FK + インデックスのスキーマを作り、出力の順序と内容が期待通りであること。

---

## Step 7: バリデーション

**やること**: `src/validate.rs` を作る。

```text
validate(&Schema) -> Vec<ValidationError>
```

**説明**

- **最初のエラーで止めず、全部集めて返す。** 画面で全部のエラーを一度に表示するため
- `ValidationError` は `thiserror` で定義した enum にする。どのテーブル・カラムのエラーか分かる情報(名前)を持たせる
- 最終的に `generate_ddl` は、エラーがなければ `Ok(Vec<String>)`、あれば `Err(Vec<ValidationError>)` を返す形に変える

**チェックする項目(優先度順に、1つずつ足していく)**

| # | チェック | 備考 |
|---|---|---|
| 1 | テーブル名・スキーマ名・カラム名が空でない | |
| 2 | 識別子が128文字以内 | 文字数(バイト数ではない)で数える |
| 3 | テーブルの重複(スキーマ名+テーブル名) | **大文字小文字を区別せず**比較する(SQL Serverの標準の照合順序に合わせる) |
| 4 | カラム名の重複(テーブル内) | 同上 |
| 5 | カラムが1つ以上ある | |
| 6 | NVARCHARの長さが1〜4000(MAXはOK) | |
| 7 | DECIMALの精度が1〜38、小数桁が0以上かつ精度以下 | |
| 8 | IDENTITYはテーブルに1つまで | |
| 9 | IDENTITYの型が整数系(Int / BigInt)であること | |
| 10 | PKのカラムが存在し、NULL不可であること | |
| 11 | インデックスのカラムが存在し、空でないこと | |
| 12 | インデックス名が同じテーブル内で重複しない | |
| 13 | FKの参照元・参照先のカラムが存在すること | |
| 14 | FKの参照元と参照先でカラム数が同じ、各ペアの型が同じであること | |
| 15 | FKの参照先がPK(またはUNIQUEインデックス)のカラムであること | |
| 16 | FK名がスキーマ全体で重複しない | |

**進め方**: 1〜5から始めて、チェックごとに「正しいスキーマはOK」「壊したスキーマはエラー」の2つのテストを書く。16個を一気に書かない。

---

## Step 8: JSON保存の往復テスト

**やること**: `Schema` をJSONにして、読み戻して、元と等しいことを確認するテストを書く。

```text
Schema → serde_json::to_string_pretty → 文字列 → serde_json::from_str → Schema
元の Schema == 読み戻した Schema
```

**確認すること**

- Step 6で使った「2テーブル + FK + インデックス」のスキーマで往復する
- 出力されたJSONを目で見て、読める形になっているか確認する(将来のファイル形式になるため)

**判断メモ**: バージョン番号付きのファイル形式(`{ "version": 1, "schema": ... }`)はPhase 3で扱う。ここでは `Schema` そのものの往復だけでよい。

---

## Step 9: 仕上げ

- [ ] `cargo fmt --check` が通る
- [ ] `cargo clippy -- -D warnings` が通る
- [ ] `cargo test` が全部通る
- [ ] 公開する関数・型だけを `pub` にする(`quote` などの内部関数は非公開のまま)
- [ ] `lib.rs` から `model` / `ddl` / `validate` を公開し、外から `artoria_core::generate_ddl` のように使えることを確認する

## Phase 1 完了条件(roadmap.mdと同じ)

テストだけで、複数テーブル・FK・インデックス付きのDDLが正しく出力される。

---

## つまずきやすい点

- **文字列比較のテストが落ちる**: ほぼ、インデントや改行(`\n`)の違い。期待値を `println!("{:?}", 出力)` で見ると差が分かる
- **FKの型一致チェック**: `SqlType` が `PartialEq` を持っていれば `==` で済む。Step 1で derive を付けておけば困らない
- **大文字小文字を区別しない比較**: 比較する前に両方を `to_lowercase()` するのが一番簡単
- **どこまで作るか迷う**: Step 7のチェックは、すべて入れなくても次に進める。1〜5だけで先に進み、後で足してもよい

## 次のフェーズ

Phase 2(設計画面)に進む前に、`Schema` を作る・編集するためのヘルパー(テーブルの追加・削除、カラムの並べ替えなど)が必要になる。Phase 1が終わったら、その設計を相談する。

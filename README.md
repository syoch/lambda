# Lambda Calculus in Rust

Rustで実装されたラムダ計算エンジンとCLIツールです。

## 特徴

- **純粋なRust実装**: 標準ライブラリのみを使用（パーサーはnomを使用）
- **CLIツール**: clapを使った使いやすいコマンドラインインターフェース
- **分離されたエンジン**: ラムダ計算ロジックは`src/lambda.rs`に分離
- **nomベースのパーサー**: 文字列からラムダ式をパース
- **de Bruijnインデックス**: 内部表現にde Bruijnインデックスを使用
- **β簡約**: 正規形への簡約をサポート
- **組み合わせ探索**: 指定した式を組み合わせて目標の式を構成できるか探索

## セットアップ

このプロジェクトは Nix Flakes と direnv を使用して環境を管理しています。

### 前提条件

- Nix（Flakes有効化）
- direnv

### 環境の有効化

```bash
# ディレクトリに入ると自動的に環境が読み込まれます
cd lambda
direnv allow

# または手動で Nix シェルを起動
nix develop
```

## ビルドと実行

```bash
# ビルド
cargo build

# リリースビルド
cargo build --release

# テスト
cargo test
```

## CLI使用方法

### ヘルプ表示

```bash
# 全体のヘルプ
cargo run -- --help

# サブコマンドのヘルプ
cargo run -- reduce --help
cargo run -- search --help
```

### 簡約化 (reduce)

ラムダ式を正規形に簡約します。

```bash
# 基本的な使い方
cargo run -- reduce '\x.x'

# 関数適用の簡約
cargo run -- reduce '(\x.x) (\y.y)'

# 詳細な簡約過程を表示
cargo run -- reduce '(\x.x) (\y.y)' --verbose

# 最大ステップ数を指定
cargo run -- reduce '(\x.\y.x y) a b' --max-steps 100
```

**オプション:**
- `-m, --max-steps <N>`: 最大簡約ステップ数（デフォルト: 1000）
- `-v, --verbose`: 簡約の各ステップを表示

### 組み合わせ探索 (search)

式 X を組み合わせて目標の式 Y を構成できるか探索します。

```bash
# 基本的な使い方
cargo run -- search -x '\x.x' -y '\x.x' -n 5

# プログレスバーなしで実行
cargo run -- search -x '\x.\y.x' -y '\x.\y.y' -n 10 --quiet

# 詳細ログ付きで実行
cargo run -- search -x '\x.x' -y '\a.\b.a' -n 8 --verbose

# 正規化のステップ数を指定
cargo run -- search -x '\x.x' -y '\x.x' -n 5 --max-steps 200
```

**オプション:**
- `-x, --base-expr <式>`: 組み合わせる基本式 X（必須）
- `-y, --target-expr <式>`: 目標となる式 Y（必須）
- `-n, --max-n <N>`: 最大組み合わせ数（デフォルト: 10）
- `-s, --max-steps <N>`: 正規化の最大ステップ数（デフォルト: 400）
- `-q, --quiet`: プログレスバーを表示しない
- `-v, --verbose`: 詳細ログを表示

### 使用例

```bash
# 恒等関数を簡約
cargo run -- reduce '\x.x'

# K コンビネータに引数を適用
cargo run -- reduce '(\x.\y.x) a b' --verbose

# S コンビネータを簡約
cargo run -- reduce '\x.\y.\z.x z (y z)'

# X を組み合わせて Y を探索
cargo run -- search \
  -x '\f. f (\g. \y. \s. g s (y s)) (\a. \b. a)' \
  -y '\x. \a. \b. x (\x. \b. b (x a)) (\y. b) (\x. x)' \
  -n 14 \
  -s 400
```

## プロジェクト構造

```
lambda/
├── flake.nix               # Nix Flakes設定
├── Cargo.toml              # Rustプロジェクト設定
├── src/
│   ├── lib.rs              # ライブラリルート
│   ├── lambda.rs           # ラムダ計算エンジン（de Bruijnインデックス）
│   ├── parser.rs           # nomベースのパーサー
│   ├── search_combination.rs  # 組み合わせ探索
│   └── main.rs             # CLIエントリーポイント
└── README.md
```

## 実装されている機能

### de Bruijn インデックス

内部表現にde Bruijnインデックスを使用しており、変数名の違いを気にせずに構造的な等価性を判定できます。

### 主要な機能

1. **β簡約** (`beta_reduce_step`, `normalize`): ラムダ式を正規形に簡約
2. **文字列表示** (`Display` trait): 読みやすい形式で表示
3. **組み合わせ探索** (`search_combination`): 式を組み合わせて目標の式を構成

### 組み合わせ探索アルゴリズム

指定したラムダ式 X を最大 n 個組み合わせて、目標の式 Y を構成できるかを探索します。

**アルゴリズム:**
1. X を n-1 個使った組み合わせから開始
2. 各組み合わせに X を適用（前方・後方）
3. 生成された組み合わせを正規形に変換して重複を除去
4. Y と同値な式が見つかったら探索終了

**機能:**
- プログレスバー表示（indicatifを使用）
- 組み合わせ構造の表示
- パフォーマンス統計の出力

## パーサー構文

- **変数**: `x`, `y`, `abc` など
- **ラムダ抽象**: `\x.x` または `λx.x`
- **複数パラメータ**: `\x y.x` は `\x.\y.x` と同じ
- **関数適用**: `f x` (左結合: `f x y` = `(f x) y`)
- **括弧**: `(\x.x) y`

### 構文例

```
\x.x                    # 恒等関数 (I combinator)
\x.\y.x                 # K combinator
\x y.x                  # K combinator (短縮形)
(\x.x) y                # 関数適用
\x.\y.\z.x z (y z)      # S combinator
```

## ライセンス

このプロジェクトは教育目的のサンプルコードです。

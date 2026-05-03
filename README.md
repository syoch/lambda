# Lambda Calculus in Rust

Rustで実装されたラムダ計算エンジンとCLIツールです。

## 特徴

- **純粋なRust実装**: 標準ライブラリのみを使用（パーサーはnomを使用）
- **CLIツール**: clapを使った使いやすいコマンドラインインターフェース
- **スクリプト実行**: .lambdaファイルで複雑なテストやライブラリ管理が可能
- **モジュールシステム**: include/from-importでコードの再利用が可能
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
cargo build --release

# インストール（推奨）
mkdir -p lambda/bin lambda/lib/lambda
cp target/release/lambda lambda/bin/
cp lib/*.lambda lambda/lib/lambda/

# パスを通す（オプション）
export PATH="$PWD/lambda/bin:$PATH"

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

### スクリプト実行 (run)

.lambdaスクリプトファイルを実行します。

```bash
# 基本的な使い方
cargo run -- run script.lambda

# 詳細ログ付きで実行
cargo run -- run script.lambda --verbose

# 最大ステップ数を指定
cargo run -- run script.lambda --max-steps 5000
```

**オプション:**
- `-m, --max-steps <N>`: 最大簡約ステップ数（デフォルト: 1000）
- `-v, --verbose`: 詳細ログを表示

#### .lambdaスクリプトファイルの形式

スクリプトファイルでは、変数定義、アサーション、簡約ステップ表示、組み合わせ探索、ファイルインクルードなど、様々な機能が使えます。

```lambda
# コメント行（#で始まる）

# 変数定義
I = \x.x
K = \x.\y.x
S = \x.\y.\z.x z (y z)

# ファイルのインクルード
from "basics.lambda" import I, K, S        # 選択的にインポート

# アサーション（テスト）
assert: I (\a.a) == \b.b
assert: K I (\c.c c) == I
assert: S K K == I

# ステップ数を指定したアサーション
assert (50): S K K == I

# 簡約ステップの表示
reduce_steps S K K
reduce_steps(20) (\x.\y.x y) (\a.a a)

# 組み合わせ探索
search K -> I                                   # デフォルト: max_n=10, max_steps=400
search(5) S -> \x.\y.x                         # max_nのみ指定
search(10, 200) K -> \x.x                      # max_nとmax_stepsを指定
```

**構文:**
- `<変数名> = <ラムダ式>`: 変数を定義
- `include "<パス>"`: 別ファイルの全定義をインポート
- `include "<パス>" as <名前空間>`: 名前空間付きでインポート（`名前空間.変数名`でアクセス）
- `from "<パス>" import <名前1>, <名前2>, ...`: 選択的にインポート
- `assert: <式1> == <式2>`: 2つの式が等価であることを検証
- `assert (<ステップ数>): <式1> == <式2>`: ステップ数を指定して検証
- `reduce_steps <式>`: 簡約ステップを表示
- `reduce_steps(<ステップ数>) <式>`: ステップ数を指定して簡約を表示
- `search <式1> -> <式2>`: 組み合わせ探索
- `search(<max_n>) <式1> -> <式2>`: max_nを指定して探索
- `search(<max_n>, <max_steps>) <式1> -> <式2>`: 両方指定して探索
- `# <コメント>`: コメント行

### メモ式 [expr]

メモ式 `[expr]` は、ラムダ式を正規化して「記憶」する機能です。これにより、繰り返される複雑な式の計算を効率化できます。

**文法:**
```lambda
[expr]  # exprは任意のラムダ式
```

**主な特性:**

1. **関数位置での自動アンラップ**
   - `[E2] E1` は `E2 E1` として簡約できる
   - メモ式が関数適用の関数部分にある場合、メモを外して処理

2. **β簡約の引数には使用不可**
   - メモ式は β 簡約の引数位置には使用できない
   - これにより、内部で重複計算を防ぐ

3. **内容の正規化**
   - `[E3] = [E4]` （E3が正規形E4に簡約化される場合）

**使用例:**

```lambda
from "basics.lambda" import S, I

# シンプルな例：メモ化された恒等関数
reduce_steps(10) [I] I
# [I] I -> I -> (\x.x)

# 複雑な式のメモ化
reduce_steps(20) (\a. a) [S I I]
# メモ式 [S I I] の内容が段々と簡約化される

# SKS コンビネータの例
reduce_steps(100) (S I I) [S I I]
# メモ式が関数位置で使用される場合、自動的に外される
```

**実装詳細:**

- DeBruijn インデックス表現では `Memo(Box<DeBruijn>)` として実装
- パーサーは `[` と `]` の括弧を認識して memo 式を生成
- 簡約エンジンはメモ式の自動アンラップと内容の正規化を処理

### 標準ライブラリ

実行ファイルが`A/bin/lambda`にある場合、`A/lib/lambda/`ディレクトリが標準ライブラリパスとして探索されます。

```bash
# ディレクトリ構造
lambda/
├── bin/
│   └── lambda              # 実行ファイル
└── lib/
    └── lambda/
        ├── basics.lambda
        ├── boolean.lambda
        ├── pair.lambda
        ├── natural_number.lambda
        └── list.lambda

# スクリプトから標準ライブラリを参照
# from "basics.lambda" import I, K, S
```

#### ライブラリの詳細

**basics.lambda** - 基本コンビネータ
- `I = \x.x` - 恒等関数（恒等元素）
- `K = \x.\y.x` - 定数関数（K コンビネータ）
- `S = \x.\y.\z.x z (y z)` - 合成関数（S コンビネータ）

これらは Turing 完全なラムダ計算を実現するための基本的なコンビネータです。

**boolean.lambda** - Church ブール値
- `True = \x.\y.x` - 真（最初の引数を返す）
- `False = \x.\y.y` - 偽（二番目の引数を返す）

**pair.lambda** - 順序対（タプル）
- `Pair = \x.\y.\f.f x y` - 順序対を作成
- `PairFirst = \p.p (\x.\y.x)` - 最初の要素を取得
- `PairSeconds = \p.p (\x.\y.y)` - 二番目の要素を取得

**natural_number.lambda** - Church 数値
- `Nat0` から `Nat9` - 0〜9 の数値定義
- `NatAdd = \m.\n.\f.\x.m f (n f x)` - 加算
- `NatMultiple = \m.\n.\f.m (n f)` - 乗算
- `NatPredecessor = \n.\f.\x.n (\g.\h.h (g f)) (\u.x) (\u.u)` - 前者
- `NatIsZero = \n.n (\x.(\x.\y.y)) (\x.\y.x)` - ゼロ判定

**list.lambda** - リスト
- `ListNil = \x.x` - 空リスト
- `ListCons = \h.\t.\s.\n.s h t n` - リスト構築（cons）
- `ListHead = \l.l (\h.\t.\n.h) ListNil` - リストの先頭を取得
- `ListTail = \l.l (\h.\t.\n.t) ListNil` - リストの残りを取得

#### 使用例

```bash
# ブール値と順序対を使ったスクリプト
cat > logic.lambda << 'EOF'
from "boolean.lambda" import True, False
from "pair.lambda" import Pair, PairFirst, PairSeconds

# ペアを作成してアクセス
P = Pair True False
assert: PairFirst P == (\x.\y.x)
assert: PairSeconds P == (\x.\y.y)
EOF

lambda run logic.lambda

# Church 数値を使ったスクリプト
cat > arithmetic.lambda << 'EOF'
from "natural_number.lambda" import Nat2, Nat3, NatAdd

# 2 + 3 を計算
assert(50): NatAdd Nat2 Nat3 == (\f.\x.f (f (f (f (f x)))))
EOF

lambda run arithmetic.lambda
```

### 使用例

```bash
# 恒等関数を簡約
lambda reduce '\x.x'

# K コンビネータに引数を適用
lambda reduce '(\x.\y.x) a b' --verbose

# S コンビネータを簡約
lambda reduce '\x.\y.\z.x z (y z)'

# X を組み合わせて Y を探索
lambda search \
  -x '\f. f (\g. \y. \s. g s (y s)) (\a. \b. a)' \
  -y '\x. \a. \b. x (\x. \b. b (x a)) (\y. b) (\x. x)' \
  -n 14 \
  -s 400

# スクリプトファイルを実行
lambda run test.lambda

# 標準ライブラリを使ったスクリプト
cat > test.lambda << 'EOF'
include "combinators.lambda"
assert: S K K == I
EOF
lambda run test.lambda
```

## プロジェクト構造

```
lambda/
├── flake.nix                # Nix Flakes設定
├── Cargo.toml               # Rustプロジェクト設定
├── src/
│   ├── lib.rs               # ライブラリルート
│   ├── lambda.rs            # ラムダ計算エンジン（de Bruijnインデックス）
│   ├── parser.rs            # nomベースのパーサー
│   ├── script.rs            # スクリプト実行エンジン
│   ├── search_combination.rs  # 組み合わせ探索
│   └── main.rs              # CLIエントリーポイント
├── lib/
│   └── combinators.lambda   # 標準コンビネータライブラリ
└── lambda/                  # インストール先ディレクトリ（推奨）
    ├── bin/
    │   └── lambda           # 実行ファイル
    └── lib/
        └── lambda/          # 標準ライブラリパス
            └── combinators.lambda
```

## 実装されている機能

### de Bruijn インデックス

内部表現にde Bruijnインデックスを使用しており、変数名の違いを気にせずに構造的な等価性を判定できます。

### 主要な機能

1. **β簡約** (`beta_reduce_step`, `normalize`): ラムダ式を正規形に簡約
2. **文字列表示** (`Display` trait): 読みやすい形式で表示
3. **組み合わせ探索** (`search_combination`): 式を組み合わせて目標の式を構成
4. **スクリプト実行** (`run_script`): .lambdaファイルの実行
5. **モジュールシステム**: ファイルのインクルードと名前空間管理

### スクリプト機能

`.lambda`ファイルでは以下の機能が使えます：

- **変数定義**: `I = \x.x`
- **アサーション**: `assert: expr1 == expr2`
- **簡約ステップ表示**: `reduce_steps expr`
- **組み合わせ探索**: `search base -> target`
- **ファイルインクルード**:
  - `include "path"` - 全定義をインポート
  - `include "path" as ns` - 名前空間付きでインポート
  - `from "path" import name1, name2` - 選択的インポート

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

## テスト

提供されているすべてのラムダ計算抽象について、包括的なテストスイートが用意されています。

### テスト実行

```bash
# すべてのライブラリテストを実行
lambda run tests/test_basics.lambda
lambda run tests/test_boolean.lambda
lambda run tests/test_pair.lambda
lambda run tests/test_natural_number.lambda
lambda run tests/test_list.lambda

# または統合テストスイートを実行
lambda run tests/test_all.lambda
```

### テスト結果（全て合格 ✓）

- **test_basics.lambda**: 6/6 合格 - I, K, S コンビネータの検証
- **test_boolean.lambda**: 4/4 合格 - True, False ブール値の検証
- **test_pair.lambda**: 6/6 合格 - Pair, PairFirst, PairSeconds の検証
- **test_natural_number.lambda**: 33/33 合格 - Church 数値と演算の検証
- **test_list.lambda**: 5/5 合格 - ListNil, ListCons, ListHead, ListTail の検証

各テストはアサーション形式で、ラムダ式の正確な動作を検証します。

## ライセンス

このプロジェクトは教育目的のサンプルコードです。

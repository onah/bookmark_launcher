# 前提・背景

- 開発環境はmacだが、通常良く利用する業務マシンはWindows
- 業務マシンは通常操作（エディタ、ブラウズ、オフィス操作）には十分なスペックを備えているが開発はしていない
  - ディスクが潤沢に利用できず、開発系作業は、別途Linuxでコード確認などを行うことが多い
  - 今回はいったん対象から外して良い
- Windwosだがターミナルも効率よく利用したい
- ファイラーやyaziを活用しているが、検索やfzf,zoxideによるディレクトリ移動の後、日本語モードになっているのが煩わしい
- エディタはこれまでvscodeやgvimを利用しているが、コンソール上のnvimに移れる可能性を探っている（軽さ優先）
- AI系ツールは現状Github Copilot(vscode)が主流だが、claude codeに移行する予定がある。
  - M365Copilotやその他AIブラウザ経由のAIチャットも活用
- cdの移動やfzf, zoxideを十分に活用したいが、フォルダは日本語フォルダが多いため、migemo検索がしたい
- bookmark もfzf類似インターフェースに流す感じで選択されたURLを開くようなものが望ましいが、URLだけでなくタイトルやタグなど（日本語含む）でも検索したい

# 設計方針

- fzfのインターフェースと類似したmigemo検索ができるツールが望ましいのではないか
- cdの移動やzoxide類似の移動が、日本語フォルダでもmigemo検索ができるようなツール
- bookmark管理もインターフェース部分は同じツールに任せ、インプットとアウトプットを加工するツール

# 決定事項（2026-10-12）

## 名前

- プロジェクト名: **kanaq**（kana + query。日本語の文字列をアルファベット（ローマ字）で問い合わせる道具）
- 実行ファイル
  - `kanaq`: migemo 系を1つに統合し、サブコマンドで分ける（`filter` / `walk` / `complete`）
  - `kanaq-bm`: ブックマーク（タイトル取得で HTTP/TLS が必要になり重いため別 exe）
- 普段打つ短い名前は、シェル側のエイリアスで付ける（yazi の `y`、zoxide の `z` と同じ考え方）
- データの保存場所: `ProjectDirs::from("com", "onah", "kanaq")`（既存の `com.onah.bookmark_launcher` から移し替える）
- リポジトリ: 新規作成はせず、今のリポジトリを作り変えて名前を変える（履歴を残す）

## 対象環境

- Windows（非力な VPC）が主対象。PowerShell は 5.1 と 7 の両方に対応する
- PowerShell 側は薄くし（fzf を起動して結果を受け取るだけ）、データの処理は Rust 側に寄せる
- キー入力ごとの負荷を小さく保つ（候補の読みは事前に作成して使い回す）

## 構成

```
kanaq/
├── Cargo.toml           workspace
├── crates/
│   ├── migemo-core/     ライブラリ: 読み・検索・辞書の読み込みとキャッシュ
│   ├── kanaq/           exe: filter / walk / complete
│   └── kanaq-bm/        exe: list / open / add
├── shell/
│   ├── powershell/      1つのモジュール (.psm1)
│   ├── zsh/
│   └── fish/
└── docs/
```

## 移行手順（各ステップを1コミット）

1. TUI（`bookmark_launcher`）の撤去。撤去前の状態にタグ `tui-final` を付ける
2. 辞書の場所の決定とキャッシュの読み込みを `migemo-core` に移す（重複の解消）
3. `crates/` と `shell/` への移動（ファイルの移動のみ）
4. `migemo-fzf` と `migemo-complete` を `kanaq` のサブコマンドに統合する
5. 名前の変更（リポジトリ名、データの保存場所の移し替え）

# 進捗（2026-10-12 時点）

上の「決定事項」は決めた当時の記録として残し、実際の状態はこの節にまとめる。

## 完了したこと

- 移行手順 1〜5 はすべて完了
  - TUI は撤去済み（最終状態はタグ `tui-final`）
  - リポジトリ名は `onah/kanaq`、データの保存場所は `com.onah.kanaq`
- `kanaq complete`（シェルの Tab 補完）を zsh の既定の動きに寄せた
  - 先頭一致を優先し、なければ部分一致
  - `.` で始まる名前は、入力が `.` で始まるときだけ候補にする
  - 読みで一致した候補に文字の共通部分がないとき（`te` → `test/` と `テスト/`）に、入力を消さない
- 検索の規則の改善（詳細は `docs/migemo_ja.md` の 6-4）
  - 数字と記号は1文字をそのまま読みとして扱う（`202610_te` → `202610_テスト`）
  - 読めない末尾に母音を含む入力はローマ字とみなさない（`github` が「定義」に一致しない）
- `kanaq-cd`：引数で検索語を渡せる（`j keihi`）。候補が1件なら fzf を開かずに移動する
- `kanaq-bm`（ブックマーク）：`list` / `path` / `open` / `add` / `remove` とシェル連携（`bm` / `bme`）
- 自動テスト：`cargo test` に加え、疑似端末で zsh を動かす `tests/zsh-completion.zsh` と `tests/zsh-bookmarks.zsh`

## 現在の構成

```
kanaq/
├── crates/
│   ├── migemo-core/     ライブラリ: 読み・検索・辞書の読み込みとキャッシュ・データの保存場所
│   ├── kanaq/           exe: filter / complete（walk は未実装）
│   └── kanaq-bm/        exe: list / path / open / add / remove
├── shell/
│   ├── powershell/      kanaq-cd.ps1 / kanaq-bm.ps1 / cd-completion.ps1（モジュール化は未実施）
│   ├── zsh/             kanaq-cd.zsh / kanaq-bm.zsh / _cd / _kanaq_files
│   └── fish/            cd.fish
├── tests/               zsh-completion.zsh / zsh-bookmarks.zsh
└── docs/
```

## kanaq-bm の設計

- 画面と検索は fzf と `kanaq filter` に任せ、`kanaq-bm` はデータの出し入れだけを行う
- データは `bookmarks.toml`。旧 TUI の形式をそのまま読める
  - 各項目の `type` 欄は残す（今はブックマークだけに対応。将来ほかの種類を足せるように）
  - タグは省略可。設定は `add --tag` で行う
- 編集（名前やタグの変更、並べ替え）は、エディタでファイルを直接開いて行う（`bme`）
  - `kanaq-bm` による更新は変える値だけを書き換えるので、手書きのコメントや書式は残る
  - 保存は一時ファイルに書いてから置き換える（途中で止まっても壊れない）
- `add` で `--title` を省くと、ページのタイトルを取得する
  - 文字コードはヘッダー → `<meta charset>` → UTF-8 の順に判断する（Shift_JIS のページにも対応）
  - 取得に失敗したらドメイン名をタイトルにする
- `open` は既定のブラウザで開き、開いた回数を数える（`list` の並び順に使う）
  - 環境変数 `KANAQ_BM_OPENER` で開くプログラムを差し替えられる（テストでも使用）
- HTTP と TLS を含むため、`kanaq` 本体とは別の exe にしている（`kanaq` 約 0.8MB、`kanaq-bm` 約 3.9MB）

# 今後の課題

- **PowerShell 5.1 への対応と Windows 実機での確認**（業務マシンで使う前に必要）
  - PowerShell のスクリプトは、開発環境（mac）に PowerShell がないため、まだ一度も動かしていない
  - 5.1 の文字コードの扱い（外部コマンドへのパイプが ASCII、`Set-Content` が CP932）と、`GetRelativePath` がない問題への対応が必要
  - `shell/powershell/` のスクリプトを1つのモジュール（`.psm1`）にまとめる
- **`kanaq walk`**（候補の生成と読みの事前作成）：未着手。Windows 実機でキー入力ごとの処理時間を測ってから要否を判断する
- **あいまい検索（fuzzy）と migemo を組み合わせた採点**
  - 途中を飛ばした入力（`2026te` で `202610_テスト`）に一致させるため
  - 子音で終わる短い入力（`test` → `te`）が、関係のない名前にも一致しやすい問題を改善するため

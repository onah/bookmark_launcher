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

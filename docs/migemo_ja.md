# 独自 Migemo 実装 設計書

## 1. 解決する問題

rustmigemo は複数モーラのクエリで漢字にマッチできない。

```
"ge"  → 「言」にマッチ ✓   （1モーラ：文字クラスに漢字を直接列挙）
"gen" → 「言」にマッチしない ✗  （2モーラ：ひらがな "げん" のみ生成）
```

原因：rustmigemo の複数モーラ処理はひらがな/カタカナ列を生成するが、
同じ読みを持つ漢字をパターンに含めない。

---

## 2. 採用アプローチ：ローマ字モーラインデックス方式

テキストを直接正規表現でマッチするのをやめ、
**各アイテムの読みをローマ字モーラのリストとして事前計算し、インデックスとして保持する**。

```
追加時: "プログラム言語"
       └→ 分かち書き+読み: ["pu","ro","gu","ra","mu"] + ["ge","n","go"]
       └→ モーラリスト:   ["pu","ro","gu","ra","mu","ge","n","go"]

検索時: "gen" → モーラ分解 → ["ge","n"]
       └→ ["pu","ro","gu","ra","mu","ge","n","go"] の連続部分列に一致 → ヒット ✓

検索時: "gengo" → ["ge","n","go"]
       └→ 同様に一致 → ヒット ✓
```

### なぜローマ字か（ひらがなでなく）

| 観点 | ひらがな保持 | ローマ字保持 |
|------|------------|------------|
| 検索時の変換 | 毎回ローマ字→ひらがな変換が必要 | **不要（ユーザー入力のまま比較）** |
| ASCII との親和性 | スクリプトが混在する | **均質なASCII文字列として扱える** |
| n の曖昧性 | 検索時に判断が必要 | **問題なし（保存時に確定済み）** |
| si/shi ゆれ | 自動吸収できる | クエリ側の正規化テーブルで対応 |

ブックマークランチャーの特性（追加は稀・検索は頻繁・タイトルにASCIIが多い）を踏まえ、
検索時の処理をゼロにできるローマ字保持を採用する。

### なぜモーラリストか（連結文字列でなく）

連結文字列 `"puroguramugengo"` では**モーラ境界をまたぐ誤マッチ**が発生する。

```
"ogu" で検索 → pur[ogu]ramugengo → 誤ヒット ✗  （"おぐ" は存在しない）
"amu" で検索 → purogur[amu]gengo → 誤ヒット ✗  （"あむ" は存在しない）
```

モーラリストでの**連続部分列マッチ**はモーラ境界でしかマッチしない。

```
["pu","ro","gu","ra","mu","ge","n","go"]
["o","gu"] → 連続部分列なし → 誤ヒットなし ✓
["ro","gu"] → 位置1-2に一致 → 正しくヒット ✓
```

---

## 3. 辞書：SKK-JISYO の活用

### フォーマット

- エンコーディング：EUC-JP（読み込み時に UTF-8 へ変換）
- 2セクションのうち「送り仮名なし」エントリを使用

```
;; okuri-nasi entries.
げん /現/言/減/源;発生-/原;-住民/元/...
げんご /言語/原語;original language/源吾/...
こと /事/言/琴;(楽器)/...
```

### 構造

```
読み(ひらがな) SP /候補1/候補2;注記/.../ LF
```

- 候補のセミコロン以降は注記（検索対象外）
- `#` で始まる数値プレースホルダは無視してよい
- 約 13 万エントリ（送り仮名なし）

### 構築するインデックス

**語 → ローマ字モーラリスト**（アイテム追加時の読み計算に使用）

辞書のひらがな読みを、ロード時にローマ字モーラへ変換して格納する。

```
"言語" → ["ge", "n", "go"]
"言"   → ["ge", "n"]
"現"   → ["ge", "n"]
"管理" → ["ka", "n", "ri"]
```

> 注：漢字は複数の読みを持つ（言 = げん / こと / い）。
> v1 では辞書スキャン順の最初の読みを採用する。精度向上は Phase 2 以降で対応。

---

## 4. データ構造

```rust
/// 辞書（SKK-JISYOから構築）
pub struct SkkDictionary {
    /// 語 → ローマ字モーラリスト（アイテム追加時の読み計算用）
    /// 例: "言語" → ["ge", "n", "go"]
    word_to_morae: HashMap<String, Vec<String>>,
}

/// アイテムの分割単位（ハイライト位置の逆引き用）
struct Segment {
    char_range: Range<usize>,   // 元テキスト上の文字位置範囲
    morae_len: usize,           // このセグメントが占めるモーラ数
}

/// インデックス済みアイテム
struct IndexedItem {
    text: String,
    morae: Vec<String>,         // ローマ字モーラリスト（検索対象）
    segments: Vec<Segment>,     // モーラ位置 → 元テキスト文字位置の逆引き
}

/// 検索結果
pub struct SearchResult {
    pub index: usize,           // add_item の登録順インデックス
    pub highlight: Vec<usize>,  // ハイライトする文字位置（char index）
}

/// メインの検索エンジン
pub struct MigemoSearcher {
    dictionary: SkkDictionary,
    items: Vec<IndexedItem>,
}
```

---

## 5. API 設計

```rust
impl SkkDictionary {
    /// EUC-JP のバイト列から辞書を構築
    pub fn from_bytes(bytes: &[u8]) -> Self;
}

impl MigemoSearcher {
    pub fn new(dict: SkkDictionary) -> Self;

    /// 検索対象テキストを登録する
    /// 登録と同時にモーラリストを計算してインデックス化する
    pub fn add_item(&mut self, text: &str);

    /// ローマ字クエリで検索する
    /// 戻り値は登録インデックスとハイライト位置のリスト
    pub fn search(&self, query: &str) -> Vec<SearchResult>;
}
```

### 使用例

```rust
let dict = SkkDictionary::from_bytes(include_bytes!("../data/SKK-JISYO.L"));
let mut searcher = MigemoSearcher::new(dict);

searcher.add_item("プログラム言語");
searcher.add_item("GitHub - ソースコード管理");
searcher.add_item("Rust Programming Language");

// "gen" → ["ge","n"] → モーラ部分列マッチ → ヒット
let results = searcher.search("gen");

// "gitkanri" → ["g","i","t","k","a","n","ri"] → ヒット
let results = searcher.search("gitkanri");
```

---

## 6. アルゴリズム詳細

### 6-1. ひらがな → ローマ字モーラ変換テーブル

SKK-JISYO のひらがな読みをローマ字モーラに変換するために使用する。
Hepburn 式を基準とし、以下を正規形とする。

```
あ→a   い→i   う→u   え→e   お→o
か→ka  き→ki  く→ku  け→ke  こ→ko
さ→sa  し→shi す→su  せ→se  そ→so
た→ta  ち→chi つ→tsu て→te  と→to
な→na  に→ni  ぬ→nu  ね→ne  の→no
は→ha  ひ→hi  ふ→fu  へ→he  ほ→ho
ま→ma  み→mi  む→mu  め→me  も→mo
や→ya           ゆ→yu           よ→yo
ら→ra  り→ri  る→ru  れ→re  ろ→ro
わ→wa                           を→wo
ん→n
が→ga  ぎ→gi  ぐ→gu  げ→ge  ご→go
...（以下同様）
きゃ→kya きゅ→kyu きょ→kyo
しゃ→sha しゅ→shu しょ→sho
ちゃ→cha ちゅ→chu ちょ→cho
...（以下同様）
っ → 次モーラの先頭子音を重ねる（っか→kka, った→tta）
ー → 直前母音を重ねる（コード→ko-o-do または kooodo は文脈依存）
```

> v1 では長音符（ー）は前の母音を繰り返す形で正規化する。

### 6-2. SKK-JISYO のパース

```
1. バイト列を EUC-JP → UTF-8 に変換
2. ";; okuri-nasi entries." 行を探してそこから読み込み開始
3. 各行をパース:
   - ";" で始まる行はコメント → スキップ
   - 形式: `{ひらがな読み} /{候補1}/{候補2};注記/.../ `
   - 候補のセミコロン以降を除去
   - ひらがな読みをローマ字モーラリストに変換（上記テーブル使用）
   - word_to_morae に未登録の候補のみ登録（先着優先）
```

### 6-3. アイテム追加時のモーラリスト計算

**入力テキストを左から文字種で分類し、貪欲最長マッチで分割する。**

```
テキスト: "GitHub - ソースコード管理"

① ASCII ラン "GitHub":
   → 各文字を1モーラとして扱い小文字化
   → morae: ["g","i","t","h","u","b"]
   → segment: { char_range: 0..6, morae_len: 6 }

② ASCII ラン " - ":
   → morae: [" ","-"," "]
   → segment: { char_range: 6..9, morae_len: 3 }

③ カタカナ ラン "ソースコード":
   → カタカナ→ひらがな変換 → "そーすこーど"
   → ひらがな→ローマ字変換 → ["so","o","su","ko","o","do"]
   → segment: { char_range: 9..15, morae_len: 6 }

④ 漢字ラン "管理":
   → word_to_morae で最長マッチ
   → "管理" → ["ka","n","ri"] ✓
   → segment: { char_range: 15..17, morae_len: 3 }

最終モーラリスト:
["g","i","t","h","u","b"," ","-"," ","so","o","su","ko","o","do","ka","n","ri"]
```

**漢字の最長マッチ:**

```
漢字ラン "言語研究" に対して:
  "言語研究" → なし
  "言語研"   → なし
  "言語"     → ["ge","n","go"] ✓ → segment記録、位置を2文字進める
  "研究"     → ["ke","n","kyu","u"] ✓ → segment記録
```

**フォールバック（辞書未収録の漢字）:**

単漢字でも辞書にない場合はその文字をスキップし、
ハイライトの対象外とする（検索にはヒットしなくなる）。

### 6-4. クエリのモーラ分解

ユーザー入力のローマ字をモーラリストに分解する。
保存側と同じ Hepburn 式テーブルを使用し、表記ゆれを正規化する。

```
"gengo"  → ["ge","n","go"]     （完全変換）
"kanri"  → ["ka","n","ri"]     （完全変換）
"si"     → ["shi"]             （si → shi に正規化）
"ti"     → ["chi"]             （ti → chi に正規化）
"geng"   → ["ge","n"] + 残余"g" （不完全モーラ）
"gen"    → ["ge","n"]          （語末 n は "n" モーラとして確定）
```

**表記ゆれ正規化テーブル（クエリ側のみ）:**

```
si→shi, ti→chi, tu→tsu, hu→fu, zi→ji, di→ji
```

**不完全モーラの扱い（v1）:**

```
"geng" → 確定モーラ ["ge","n"] のみで検索する
```

### 6-5. 検索とハイライト

**マッチング：連続部分列（ウィンドウ）探索**

```rust
fn is_match(item_morae: &[String], query_morae: &[String]) -> Option<usize> {
    item_morae
        .windows(query_morae.len())
        .position(|w| w == query_morae)
}
```

**ハイライト位置の逆引き:**

```
クエリ "gengo" → ["ge","n","go"] → モーラ位置 5..8 でマッチ

segments を走査してモーラ累積位置を追跡:
  Segment 0: char_range=0..5,  morae_len=5 → 累積0..5  → モーラ5はここではない
  Segment 1: char_range=5..7,  morae_len=3 → 累積5..8  → モーラ5..8 がここに含まれる

元テキストの char 5..7（"言語"）をハイライト対象とする
```

---

## 7. 実装フェーズ

### Phase 1：コア実装（MVP）

- [ ] SKK-JISYO パーサ（EUC-JP変換・okuri-nasi セクション読み込み）
- [ ] ひらがな→ローマ字モーラ変換テーブル
- [ ] `SkkDictionary::from_bytes`
- [ ] `MigemoSearcher::add_item`（ASCII/カタカナ/ひらがな/漢字の各文字種対応）
- [ ] ローマ字クエリのモーラ分解（表記ゆれ正規化含む）
- [ ] `MigemoSearcher::search`（連続部分列マッチ + ハイライト逆引き）
- [ ] 既存の `MigemoEngine` を置き換えて動作確認

### Phase 2：精度改善

- [ ] 送り仮名あり（okuri-ari）エントリを使った動詞活用対応
- [ ] 不完全モーラの前方一致フィルタ（"geng" で "げんご" にもヒット）
- [ ] 多音字の複数読み対応（"言" を "げん" と "こと" 両方でヒット）

### Phase 3：パフォーマンス

- [ ] 辞書データのバイナリシリアライズ（毎回パースしない）
- [ ] 読み計算のキャッシュ

---

## 8. 制約・既知の問題

| 項目 | 内容 |
|------|------|
| 多音字 | 「言」は"げん"/"こと"/"い"など複数読みがある。v1 は辞書の先着順で1つに固定するため、正読み以外では検索不可 |
| 未収録語 | SKK-JISYO に未収録の固有名詞・新語は漢字のまま読み不明になりスキップされる |
| 不完全モーラ | "geng" 入力時は "ge","n" のみで検索（Phase 2 で前方一致を追加予定） |
| 送り仮名 | "読む" の活用形などは Phase 2 対応 |
| 長音符 | "ソース" → "so","o","su" のように母音繰り返しで正規化するため、"so-su" では検索不可 |

# SVG Stroke（Premiere Pro / After Effects 用 多重縁取り）

クリップのアルファから、太さ・色・不透明度を指定できる縁取りを最大3重に生成するエフェクトです。

## 状態（重要）
- `src/core.rs`（縁取りの計算）の距離変換だけは、同じ式をPythonで再現して動作確認済み。
- `src/lib.rs`（Premiere/AEとの接続部分）は **未コンパイル・未実機テスト**。
  `after-effects` クレートのバージョン差でAPI名の修正が必要になる可能性があります。
- この環境ではmacOS用バイナリを作れないため、ビルドはGitHub Actionsで行います。

## ビルド
1. このフォルダをGitHubにpush（`.github/workflows/build.yml` が自動で走ります）
2. Actionsの成果物 `SVG_Stroke_mac`（.plugin） / `SVG_Stroke_win`（.aex）をダウンロード
3. 置き場所
   - macOS: `/Library/Application Support/Adobe/Common/Plug-ins/7.0/MediaCore/`
   - Windows: `C:\Program Files\Adobe\Common\Plug-ins\7.0\MediaCore\`
4. Premiereを再起動 → エフェクト「Stylize > SVG Stroke」

## 既知の限界 / 次の段階
- 斜め方向の縁が最大約0.5pxずれる（ピクセル単位の距離のため）。アルファ値を使ったサブピクセル補正で改善可能。
- 「角:ミター」は未実装（現在は丸のみ）。
- 本物のSVGパス（ベクター）入力には未対応。必要なら別途パスパラメータを追加します。
- GPU（Metal/CUDA）対応なし。重い場合はCPUのrayon並列化から。

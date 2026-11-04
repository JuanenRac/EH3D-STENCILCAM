<p align="center">
  <img src="images/EH3D_BANNER.svg" alt="EH3D-STENCILCAM banner" width="100%">
</p>

# <img src="images/brand/icon-64.png" alt="" width="40" height="40" align="absmiddle"> EH3D-STENCILCAM
<p align="center"><a href="README.md">🇺🇸 English</a> | <a href="README_spa.md">🇪🇸 Español</a> | <a href="README_fra.md">🇫🇷 Français</a> | <a href="README_ita.md">🇮🇹 Italiano</a> | <a href="README_deu.md">🇩🇪 Deutsch</a> | <a href="README_zho.md">🇨🇳 中文</a> | 🇯🇵 <b>日本語</b> | <a href="README_por.md">🇵🇹 Português</a></p>

### ペースト層からのはんだペースト用ステンシル：開口、比率の確認、切断パス

<p align="center">
  <img src="https://img.shields.io/badge/License-GPL%203.0--or--later-blue.svg" alt="GPL 3.0 or later">
  <img src="https://img.shields.io/badge/Stack-Rust-DEA584.svg" alt="Rust">
  <img src="https://img.shields.io/badge/Languages-8-00E5FF.svg" alt="Languages">
  <img src="https://img.shields.io/badge/Version-0.0.4-0f6cbd.svg" alt="Version">
  <img src="https://img.shields.io/badge/Maturity-scaffolding-FFB020.svg" alt="Maturity">
</p>

---

**正直さの確認 - 現在実際に動くもの：** Rust ライブラリとコマンドライン プログラム（`eh3d-stencilcam`）で、コンピューター上で実行される 72 件のテストがあります：ペースト層のパッドをステンシルの開口に変換（縮小、丸め、レーザーのカーフ補正、ペースト離れの検査）し、G コードまたは HP-GL として書き出します。ウィンドウはまだなく、機械では試しておらず、実際のファイルの集まりでも確認していません。

---

> **安全とセキュリティの状況：** このプログラムは機械を制御せず、安全システムでもありません。出力は、機械で何かを実行する前に人が確認する必要があります。ここにあるものは実機で動かしたことがありません。


## 📋 ひと目で

| | |
|---|---|
| バージョン | `0.0.4` |
| 成熟度 | 足場 |
| テスト | 72 |
| 技術スタック | Rust |
| 実機での動作 | 未試験：ここにあるものは機械で動かしたことがありません |

**移動先：** [概要](#-概要) · [アーキテクチャ](#-アーキテクチャ) · [クイックスタート](#-クイックスタート) · [テスト](#-テスト) · [構成](#-リポジトリの構成) · [関連](#-関連プロジェクト)

## 🚦 プロジェクトの状況

各行はこのリポジトリの一部分で、信号は今日どこまで進んでいるかを示します。計画ではなく、リポジトリが示せるもの（テスト、ビルド、実行）に基づいて書かれています。

🟢 完了・検証済み · 🟡 動作する（既知の制限あり） · 🟠 一部のみ、または未検証 · 🔴 未実装または動作しない

| 領域 | 状態 | ここでの意味 |
|---|---|---|
| コア | 🟡 | ペースト層のパッドをステンシルの開口に変換（縮小、丸め、レーザーのカーフ補正、ペースト離れの検査）し、G コードまたは HP-GL として書き出します。コンピューター上で通常入力と破損入力でテスト済み。実際のファイルの集まりでは未確認。 |
| コマンドライン | 🟡 | `eh3d-stencilcam` はライブラリと同じ関数を実行して結果を表示します。エラーはメッセージと 0 以外の終了コードで返ります。 |
| テスト | 🟡 | 72 件のテストが通ります（`cargo test`）。通常ケース、境界ケース、破損入力を含みます。 |
| セキュリティ | 🟡 | 安全な Rust のみ（`forbid(unsafe_code)`）。受け取ったファイルでパニックせず、不正な値は理由付きのエラーになります。独立したレビューはありません。 |
| ウィンドウ | 🔴 | 予定：スイート共通の egui ウィンドウ。現在はコマンドラインのみです。 |
| 実機 | 🔴 | 未試験：ここにあるものは機械で実行したことがありません。 |
| ドキュメント | 🟡 | 8 言語の README、変更履歴、`docs/` から描いた 2 枚の図。 |

## 🎯 概要

* **開口**（`src/openings.rs`）：ペースト層でフラッシュされた各パッド（長方形、円、長円。ファイルの単位に関係なくミリメートル）が開口になり、材料ごとの割合で縮小され、穴が要求どおりの寸法になるようレーザーのカーフ分だけ切断線を小さくします。下面用ステンシルはミラー反転できます。
* **比率**（`src/openings.rs`）：アスペクト比（短辺÷板厚、1.5 以上）と面積比（開口面積÷壁面積、0.66 以上）で、ペーストが抜けるかを判定します。満たさない開口は位置付きで報告され、確認するまで（または `--force` まで）`convert` は書き出しません。
* **パス**（`src/output.rs`、`src/profile.rs`）：角丸の長方形、円、長円を、近い順に巡る閉じたパスにします。レーザー用は一定出力で移動中は消灯する G コード、プロッター用は 1 mm あたり 40 単位の HP-GL。Mylar、Kapton、鋼のプロファイル（厚さ、縮小、角の半径、カーフ）は出発点です。
* **不完全なステンシルは書き出されません：** レイヤーに開口にできないパッド・トラック・リージョン（ポリゴン、マクロ、リージョン）がある場合、`--force` を付けない限り `convert` はファイルを書きません。
* **レイヤーはバイト列から読み込まれます：** `--max-bytes N` は上限（既定 256 MiB）を超えるファイルを拒否し、UTF-8 でないテキストは警告付きで Latin-1 として読みます。リーダーの警告とエラーは安定したコード付きで表示され、`--hide CODE[,CODE]` でそのコードの通知を省けます。リーダーのエラーはレイヤーの一部が失われたものとして数えられ、`--force` がない限り `convert` は書き出しません。開口は EH3D-GERBERVIEWER のアパーチャ変換から作られ、多角形パッドは円として切られます（注記が付くため、これも `--force` がない限り `convert` は拒否します）。 `--arc-tolerance MM` は円弧をどれだけ忠実にたどるかを決め、大きすぎるファイルの拒否メッセージはシステムの言語で表示されます。

## 🧩 EH3D での役割

EH3D-STENCILCAM は EH3D Fabrication Suite のプログラムの一つです。現在はライブラリとコマンドラインとして動作し、スイートの他のプログラムと共通の外観のウィンドウは予定です。単純なファイルを読み書きするため、単独でも他のプログラムの間でも使えます。

## 🏛️ アーキテクチャ

<p align="center">
  <img src="images/architecture.svg" alt="EH3D-STENCILCAM のアーキテクチャ：誰が使い、何を持ち、何に依存するか" width="100%">
</p>


このリポジトリの各部分の関係：誰が使い、何を持ち、何に依存するか。図は `docs/architecture.json` から描かれ、コードと照合されます。

## ✨ 機能

* **ライブラリとコマンドライン：** `eh3d-stencilcam` の各コマンドはライブラリの関数なので、スイートの他のプログラムがプロセスを起動せずに呼び出せます。
* **厳密なコマンドライン：** 書き間違えたオプション（`--powr`）や余分な語は無視されず、それを名指しするエラーになります。そうでないと、プログラムは既定値で出力を作ってしまいます。
* **破損入力のテスト：** 空のファイル、ゴミデータ、巨大な数値、途中で切れたファイルも与え、どれもパニックしてはいけません。
* **推測しない：** 意味のない値は、どの値かと理由を示すエラーになり、見かけだけ正しい結果にはなりません。
* **unsafe コードなし：** ライブラリとプログラムに `#![forbid(unsafe_code)]`。
* **G コードに標準のヘッダーを付け、順序づけを速くしました：** ファイルの先頭にジェネレーター、単位、レイヤーとその SHA-256 のコメント行が付き、角が鋭い長方形に重複する点はなく、切断の順序はグリッドで求めるので 10 万個の開口も一瞬で並べ替えられます。

## 🔒 セキュリティと安全

| 脅威 | 対策 |
|---|---|
| 破損・途中切れ・空のファイル | パニックなし：テストは有効な入力をあらゆる位置で切り、ゴミデータも与えます |
| 意味のない数値（負、無限、極端に大きい） | 値を示すエラーで拒否され、もっともらしい値に丸められることはありません |
| 安全でないメモリ使用 | `forbid(unsafe_code)` の安全な Rust のみ |

**防御していないこと：** エラーなく作られた結果は、検証された結果ではありません。ここには機械の限界を知るものはなく、プログラムは独立したレビューを受けていません。脆弱性は非公開で報告してください（[SECURITY.md](SECURITY.md) 参照）。

## 🔌 インターフェース

ライブラリは `eh3d_stencilcam`（`use eh3d_stencilcam::...`）、コマンドラインは `eh3d-stencilcam` です。コマンドは次のとおりで、結果は標準出力に表示され、エラーは標準エラー出力に 0 以外の終了コードで報告されます。ネットワークアクセスはありません。

```text
EH3D-STENCILCAM/
├── src/
│   ├── lib.rs                                   ライブラリ：各コマンドが使うもの
│   ├── main.rs                                  コマンドライン
│   ├── cli_args.rs                              コマンドライン引数：小さなツールすべてで同じファイル
│   ├── layer.rs                                 バイト列からレイヤーを読み、リーダーの通知をコードで表示
│   ├── openings.rs                              ロジックの一部
│   ├── output.rs                                ロジックの一部
│   └── profile.rs                               ロジックの一部
├── tests/
│   ├── reading.rs                               ファイル読み込みのテスト：サイズ上限、UTF-8以外のテキスト、コード別の通知
│   ├── readme.rs                                READMEとrunスクリプトが約束する内容を実行するテスト
│   ├── stencil.rs                               ライブラリとコマンドラインのテスト
│   └── strict.rs                                コマンドラインのテスト：知らないオプションと語
├── examples/
│   └── paste.gtp                                試すためのサンプル
├── docs/
│   ├── architecture.json                        アーキテクチャ図のデータ
│   └── structure.json                           構成図のデータ
├── tools/                                       CI と README のチェック、図のツール
├── images/                                      バナーとアーキテクチャ・リポジトリ構成の図
├── .github/                                     CI とリリースのワークフロー、Issue と PR のテンプレート
├── eh3d.project.json                            マニフェスト：バージョン、役割、成熟度、ビルドコマンド
├── Cargo.toml Cargo.lock                        ワークスペースと固定された依存関係
├── rust-toolchain.toml                          固定されたRustツールチェーン
├── CHANGELOG.md                                 各バージョンの変更点
├── SECURITY.md SUPPORT.md                       脆弱性の報告方法と問い合わせ先
└── CONTRIBUTING.md CODE_OF_CONDUCT.md LICENSE   貢献方法、行動規範、ライセンス
```

## ⚙️ 設定

設定ファイルはまだありません。選択は（インターフェースを参照）コマンドラインの引数で渡し、各既定値はコマンドのヘルプ（`eh3d-stencilcam help`）に書かれています。固定の上限はコード内の名前付き定数で、テストで確認されています。

## 🚀 クイックスタート

安定版の Rust ツールチェーン（1.75 以降）が必要です。

```bash
cargo build --release
cargo test
cargo run --release -- check examples/paste.gtp
```

## 🧪 テスト

テストは 72 件あります（`cargo test`、CI では Windows と Linux で実行）。通常ケース、境界ケース、破損入力を確認し、空のファイル、ゴミデータ、途中で切れたファイルでもパニックしません。CI は `cargo fmt --check`、警告をエラーにした `cargo clippy`、`python -I tools/ci_validate.py` も実行します。

## 🏗️ ビルドとリリース

バージョンは `Cargo.toml` と `eh3d.project.json` にあり、各バージョンは [CHANGELOG.md](CHANGELOG.md) に項目があります。`build.bat` / `build.sh` でビルド、`test.bat` / `test.sh` でテスト、`run.bat` / `run.sh` でサンプル実行します。リリースは GitHub で公開されます。

## 📂 リポジトリの構成

<p align="center">
  <img src="images/repository-structure.svg" alt="EH3D-STENCILCAM のリポジトリ構成" width="100%">
</p>

<details>
<summary>ツリーのテキスト版</summary>

```text
EH3D-STENCILCAM/
├── src/
│   ├── lib.rs                                   ライブラリ：各コマンドが使うもの
│   ├── main.rs                                  コマンドライン
│   ├── cli_args.rs                              コマンドライン引数：小さなツールすべてで同じファイル
│   ├── layer.rs                                 バイト列からレイヤーを読み、リーダーの通知をコードで表示
│   ├── openings.rs                              ロジックの一部
│   ├── output.rs                                ロジックの一部
│   └── profile.rs                               ロジックの一部
├── tests/
│   ├── reading.rs                               ファイル読み込みのテスト：サイズ上限、UTF-8以外のテキスト、コード別の通知
│   ├── readme.rs                                READMEとrunスクリプトが約束する内容を実行するテスト
│   ├── stencil.rs                               ライブラリとコマンドラインのテスト
│   └── strict.rs                                コマンドラインのテスト：知らないオプションと語
├── examples/
│   └── paste.gtp                                試すためのサンプル
├── docs/
│   ├── architecture.json                        アーキテクチャ図のデータ
│   └── structure.json                           構成図のデータ
├── tools/                                       CI と README のチェック、図のツール
├── images/                                      バナーとアーキテクチャ・リポジトリ構成の図
├── .github/                                     CI とリリースのワークフロー、Issue と PR のテンプレート
├── eh3d.project.json                            マニフェスト：バージョン、役割、成熟度、ビルドコマンド
├── Cargo.toml Cargo.lock                        ワークスペースと固定された依存関係
├── rust-toolchain.toml                          固定されたRustツールチェーン
├── CHANGELOG.md                                 各バージョンの変更点
├── SECURITY.md SUPPORT.md                       脆弱性の報告方法と問い合わせ先
└── CONTRIBUTING.md CODE_OF_CONDUCT.md LICENSE   貢献方法、行動規範、ライセンス
```

</details>

## 🗺️ ロードマップと既知の制限

**現時点で分かっている制限**

* ウィンドウはまだなく、ライブラリとコマンドラインだけがあります。
* 機械でも実際のファイルの集まりでも試しておらず、独立したレビューも受けていません。
* ペースト層のマクロパッド・配線・領域は開口にならず（多角形パッドは円として切られ）、材料プロファイルは出発点で、鋼板向けの多パス切断はありません。

**次のステップ**

* 実際のペースト層とレーザーで試し、工具庫からプロファイルを読み、開口の表示付きウィンドウを追加する。
* スイート共通の外観で 8 言語のウィンドウを追加する。
* 実際のファイルとスイートの他のプログラムで試す。

## 🌍 言語

README とテキストは、英語、スペイン語、フランス語、イタリア語、ドイツ語、中国語、日本語、ヨーロッパポルトガル語で提供されています。

## 🔗 関連プロジェクト

* [EH3D-CORE-LIBS](https://github.com/JuanenRac/EH3D-CORE-LIBS): Gerber、Excellon、G コードのリーダー
* [EH3D-PHOENIX-LASER](https://github.com/JuanenRac/EH3D-PHOENIX-LASER): レーザー彫刻と切断
* [EH3D-GERBERVIEWER](https://github.com/JuanenRac/EH3D-GERBERVIEWER): Gerber レイヤー ビューアー
* [EH3D-TOOL-MANAGER](https://github.com/JuanenRac/EH3D-TOOL-MANAGER): CAM プログラム共通の工具庫
* [EH3D-CAM-DEBUGGER](https://github.com/JuanenRac/EH3D-CAM-DEBUGGER): 機械に届く前の G コード検査

## 📚 ドキュメント

* [変更履歴](CHANGELOG.md)
* [アーキテクチャと構成のデータ](docs/architecture.json)：2 枚の図を描くためのデータ。

## 🤝 コントリビュートとコミュニティ

* プルリクエストの前に `cargo fmt --check`、`cargo clippy --all-targets -- -D warnings`、`cargo test`、`python -I tools/ci_validate.py` を実行し、すべて通してください。[CONTRIBUTING.md](CONTRIBUTING.md) と[行動規範](CODE_OF_CONDUCT.md)を参照。
* README は 8 言語あります。文章の変更は 8 ファイルすべてで行い、2 枚の図は `tools/make_architecture.py` と `tools/make_structure.py` が `docs/architecture.json` と `docs/structure.json` から描きます。
* 脆弱性は非公開で報告してください：[SECURITY.md](SECURITY.md)。質問・提案・報告：electrohobby3d@gmail.com（[SUPPORT.md](SUPPORT.md)）。

## 👤 作者

**JuanenRac (Electro Hobby 3D)** · electrohobby3d@gmail.com

## 📜 ライセンス

ソフトウェア：GPL-3.0-or-later。[LICENSE](LICENSE) を参照。スイートのハードウェア設計には CERN OHL-S を使います。

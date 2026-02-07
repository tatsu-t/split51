# tatsu-audioapp

Windowsのメインオーディオ出力からサラウンドチャンネルをキャプチャし、別のオーディオデバイスにルーティングするツール。

## 機能

- WASAPI Loopbackでプライマリスピーカー（4ch以上）からキャプチャ
- マルチチャンネルオーディオからRL（リアレフト）とRR（リアライト）チャンネルを抽出
- 抽出したチャンネルをセカンダリ出力デバイスへルーティング
- サンプルレート自動変換（リサンプリング）
- システムトレイアプリ（右クリックで設定メニュー）
- チャンネル別の音量、ミュート、ソース選択
- L/R入れ替えとバランス調整
- スピーカーテストトーン
- 設定の永続化（TOML）

## 使用例

このツールは以下のような環境で使用します:
- 4ch以上のオーディオ出力がある（例: Realtekの4ch/5.1ch設定）
- リアサラウンドチャンネルを別の物理出力にルーティングしたい
- 仮想オーディオデバイスを増やしたくない

例: メインのSpeakers出力からリアスピーカーをRealtek 2nd Outputにルーティング。

## 必要条件

- Windows 10/11
- 4ch以上対応のオーディオデバイス
- セカンダリオーディオ出力デバイス
- Rustツールチェーン（ビルドする場合）

## ビルド

```powershell
cargo build --release
```

実行ファイルは `target\release\tatsu-audioapp.exe` に生成されます。

## 使い方

1. `tatsu-audioapp.exe` を実行
2. アプリはシステムトレイで起動
3. トレイアイコンを右クリックして設定にアクセス:
   - **Enable/Disable Routing** - オーディオルーティングの開始/停止
   - **Swap L/R Channels** - 左右チャンネル入れ替え
   - **Source Device** - キャプチャ元デバイス（ループバック）
   - **Target Device** - 出力デバイス
   - **Master Volume** - 全体音量
   - **Balance** - 左右バランス調整
   - **Left/Right Speaker** - チャンネル別設定（ソース、音量、ミュート）
   - **Speaker Test** - 各スピーカーのテストトーン

## コマンドラインオプション

```
tatsu-audioapp --help     # ヘルプ表示
tatsu-audioapp --version  # バージョン表示
tatsu-audioapp --list     # デバイス一覧
tatsu-audioapp --quiet    # 静かに起動
```

## 設定ファイル

設定は実行ファイルと同じディレクトリの `config.toml` に保存されます:

```toml
source_device = "Speakers (Realtek(R) Audio)"
target_device = "Realtek HD Audio 2nd output (Realtek(R) Audio)"
volume = 1.0
balance = 0.0
enabled = true
swap_channels = false

[left_channel]
source = "RL"
volume = 1.0
muted = false

[right_channel]
source = "RR"
volume = 1.0
muted = false
```

## 技術詳細

- WASAPI Loopbackで低遅延オーディオキャプチャ
- チャンネルマッピング: FL(0), FR(1), RL(2), RR(3)
- ソースとターゲットのサンプルレートが異なる場合は自動リサンプリング（例: 192kHz → 48kHz）
- キャプチャと再生間のスレッドセーフなオーディオ転送にリングバッファを使用

## 5.1ch対応アプリケーション

Windowsで本当のサラウンドサウンドを出力するアプリケーション:

**ストリーミング:**
- Netflix（Windows Storeアプリ）
- Disney+（Windows Storeアプリ）
- Plex, Jellyfin, Kodi

**ゲーム:**
- ほとんどのPCゲームは5.1chをネイティブサポート

**メディアプレーヤー:**
- VLC
- MPC-BE
- mpv

**非対応:**
- Webブラウザ（Chrome, Edge, Firefox）- HTML5オーディオはステレオのみ
- Spotify, Amazon Music（Windows版）

## ライセンス

MIT

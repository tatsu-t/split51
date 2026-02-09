# split51

Windows 5.1chサラウンドオーディオスプリッター - メインオーディオ出力からRL/RRチャンネルをキャプチャし、セカンダリ出力デバイスにルーティング。

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
- Windows起動時の自動起動

### DSP機能 (v2.0+)

- **ディレイ補正** (0-200ms) - フロントとリアのタイミング調整
- **3バンドEQ** (Low/Mid/High ±12dB) - リアスピーカーの特性補正
- **擬似サラウンド (Upmix)** - ステレオ音源をリアにも出力
- **マスター音量/ミュート同期** - Windowsの音量ミキサーと連動

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

## 制限事項

- **Bluetoothデバイス**: Bluetoothオーディオデバイス（イヤホン、スピーカー）をソースデバイスとして使用した場合、WASAPI Loopbackが正常に動作しない場合があります。これはWindowsのオーディオスタックの制限によるものです。有線またはUSBオーディオデバイスを推奨します。

## ビルド

```powershell
cargo build --release
```

実行ファイルは `target\release\split51.exe` に生成されます。

## 使い方

1. `split51.exe` を実行
2. アプリはシステムトレイで起動
3. トレイアイコンを右クリックして設定にアクセス:
   - **Enable/Disable Routing** - オーディオルーティングの開始/停止
   - **Swap L/R Channels** - 左右チャンネル入れ替え
   - **Start with Windows** - Windows起動時に自動起動
   - **Source Device** - キャプチャ元デバイス（ループバック）
   - **Target Device** - 出力デバイス
   - **Master Volume** - 全体音量
   - **Balance** - 左右バランス調整
   - **Left/Right Speaker** - チャンネル別設定（ソース、音量、ミュート）
   - **Speaker Test** - 各スピーカーのテストトーン

## コマンドラインオプション

```
split51 --help     # ヘルプ表示
split51 --version  # バージョン表示
split51 --list     # デバイス一覧
split51 --quiet    # 静かに起動
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

### アーキテクチャ

```
┌─────────────────┐    WASAPI Loopback    ┌─────────────────┐
│  Main Speakers  │ ───────────────────▶  │    split51      │
│  (4ch output)   │                       │                 │
│ FL│FR│RL│RR     │                       │  RL/RR抽出      │
└─────────────────┘                       │  + リサンプリング│
                                          └────────┬────────┘
                                                   │
                                                   ▼
                                          ┌─────────────────┐
                                          │  2nd Output     │
                                          │  (Stereo L/R)   │
                                          └─────────────────┘
```

### 処理フロー

1. **WASAPI Loopback キャプチャ**
   - Windows Audio Session API (WASAPI) を使用
   - 共有モードでメイン出力デバイスをループバックキャプチャ
   - フラグ: `AUDCLNT_STREAMFLAGS_LOOPBACK | AUDCLNT_STREAMFLAGS_EVENTCALLBACK`

2. **チャンネル抽出**
   - 4chオーディオ: FL(0), FR(1), RL(2), RR(3)
   - 5.1ch: FL(0), FR(1), FC(2), LFE(3), RL(4), RR(5)
   - RL/RRのみを抽出してステレオL/Rにマッピング

3. **リサンプリング** (必要時のみ)
   - `rubato` クレートを使用
   - SincInterpolation (Linear) による高品質変換
   - 例: 192kHz → 48kHz の自動変換

4. **出力**
   - `cpal` クレートでセカンダリデバイスに出力
   - `ringbuf` によるスレッドセーフなオーディオ転送

### 使用ライブラリ

| クレート | 役割 |
|---------|------|
| `windows` | WASAPI Loopback API |
| `cpal` | クロスプラットフォームオーディオ出力 |
| `rubato` | 高品質リサンプリング |
| `ringbuf` | ロックフリーリングバッファ |
| `parking_lot` | 高速RwLock |
| `tray-icon` | システムトレイアイコン |
| `muda` | ネイティブメニュー |
| `winit` | Windowsメッセージポンプ |
| `toml` / `serde` | 設定ファイル永続化 |

### デバッグ情報

- バッファオーバーフロー発生時はログに警告出力
- `RUST_LOG=info` 環境変数で詳細ログ有効化

## ライセンス

MIT

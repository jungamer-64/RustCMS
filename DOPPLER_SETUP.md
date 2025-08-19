# Doppler統合ガイド

Rustバックエンドは環境変数の管理にDopplerを使用できるようになりました。

## 🔐 Dopplerセットアップ

### 1. Doppler CLIのインストール

Windows PowerShell:

```powershell
# Scoopを使用（推奨）
scoop install doppler

# または直接ダウンロード
Invoke-WebRequest -Uri https://releases.doppler.com/latest/windows/amd64/doppler.exe -OutFile doppler.exe
```

### 2. Dopplerへの認証

```bash
doppler login
```

### 3. プロジェクトの設定

```bash
# プロジェクト作成（Dopplerダッシュボードで行うことも可能）
doppler projects create cms

# 開発環境の設定
doppler setup --project cms --config dev
```

### 4. 環境変数の設定

DopplerダッシュボードまたはCLIで以下の環境変数を設定：

```bash
# サーバー設定
doppler secrets set HOST=127.0.0.1
doppler secrets set PORT=3001

# データベース設定
doppler secrets set DATABASE_URL=postgres://user:pass@localhost:5432/rust_cms
doppler secrets set DATABASE_NAME=cms_production

# JWT設定
doppler secrets set JWT_SECRET=your_super_secure_jwt_secret_key
doppler secrets set JWT_EXPIRATION_HOURS=24
doppler secrets set JWT_REFRESH_EXPIRATION_DAYS=7

# CORS設定
doppler secrets set ALLOWED_ORIGINS=http://localhost:3000,https://your-domain.com

# アップロード設定
doppler secrets set UPLOAD_DIR=./uploads
```

## 🚀 サーバーの起動

### Doppler経由での起動（推奨）

```bash
# バッチファイルを使用
./start-with-doppler.bat

# PowerShellスクリプトを使用
./start-with-doppler.ps1

# 直接実行
doppler run -- cargo run
```

### フォールバック起動（.envファイル）

Dopplerが利用できない場合、自動的に`.env`ファイルにフォールバックします：

```bash
cargo run
```

## 🔧 設定管理

### 環境別設定

- **development**: `doppler setup --project cms --config dev`
- **staging**: `doppler setup --project cms --config stg`
- **production**: `doppler setup --project cms --config prd`

### セキュリティの特徴

1. **機密情報の暗号化**: Dopplerがすべての環境変数を暗号化
2. **アクセス制御**: チームメンバーごとの権限管理
3. **監査ログ**: すべての変更が記録される
4. **フォールバック**: Dopplerが利用できない場合の自動`.env`使用

### ローカル開発での使用

```bash
# 現在の設定確認
doppler configs

# 環境変数の確認
doppler secrets

# 特定の変数の確認
doppler secrets get JWT_SECRET
```

## 🎯 メリット

- ✅ **セキュリティ**: 機密情報をソースコードから分離
- ✅ **チーム協業**: 統一された環境変数管理
- ✅ **環境別管理**: dev/staging/prodの簡単な切り替え
- ✅ **自動フォールバック**: Dopplerが利用できない場合の対応
- ✅ **監査機能**: 設定変更の追跡

## 🐛 トラブルシューティング

### Doppler CLIが見つからない場合

```bash
# PATHの確認
echo $env:PATH

# Doppler CLIの再インストール
scoop uninstall doppler
scoop install doppler
```

### 認証エラーの場合

```bash
# 再ログイン
doppler logout
doppler login
```

### プロジェクト設定エラーの場合

```bash
# 現在の設定確認
doppler configure

# プロジェクト再設定
doppler setup --project cms --config dev --no-interactive
```

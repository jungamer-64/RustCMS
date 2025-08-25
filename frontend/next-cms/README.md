# Next CMS Frontend

Rust CMS API のためのシンプルなNext.jsフロントエンドです。API差異（素のJSON/ApiResponse包）に対応し、Next内API経由のプロキシでCORS問題を回避します。

## 必要条件
- Node.js 18+

## セットアップ

1. 依存関係のインストール

```
npm install
```

2. APIベースURLを設定（省略時は http://localhost:3000/api/v1）

```
echo NEXT_PUBLIC_API_BASE_URL=http://localhost:3000/api/v1 > .env.local
```

3. 開発サーバ起動（ポート4000）

```
npm run dev
```

4. ブラウザで http://localhost:4000 を開く

## ページ
- /posts: 投稿一覧（ページネーション）
- /posts/[id]: 投稿詳細

## 仕組み
- app/api/proxy/[...path]/route.ts がバックエンドへリバースプロキシ
- lib/api.ts が包み/素のJSON両対応のフェッチを提供

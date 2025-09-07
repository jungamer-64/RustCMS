# Biscuit 認証ガイド

このドキュメントでは本 CMS バックエンドでの Biscuit (v6) 利用モデルと、JWT とのデュアル運用パターンをまとめます。

## 目的

- JWT: セッション/短期アクセストークン (高速発行・UI フレンドリー)
- Biscuit: 細粒度ケイパビリティ (権限委譲, 一時的制限, オフライン検証)

## トークン構造 (概念)

```text
Authority block: base facts (user_id, role, session_id)
User blocks (任意): エージェント/サービスへ再委譲された制限付き権限
Attenuation: caveats で操作・リソース・期間を制限
```

上記は概念レベル。実際には Authority block でベースファクトを定義し、後続 block で権限を「狭める」方向にのみ変更します。

## 推奨ファクト (facts) 一覧

| Fact | 用途 | 例 |
|------|------|----|
| user_id($uuid) | 恒久ユーザ識別 | `user_id("550e8400-e29b-41d4-a716-446655440000")` |
| role($role) | RBAC 互換 | `role("admin")` |
| session($sid) | セッション追跡 | `session("abc123")` |
| version($n) | refresh ローテ管理 | `version(5)` |
| cap($domain,$action,$res) | ケイパビリティ | `cap("posts","write","*")` |

## Caveat (check) パターン例

| Caveat | 説明 | 例 |
|--------|------|----|
| time_before($ts) | 有効期限 (UNIX epoch) | `time_before(1736000000)` |
| allow_domain($d) | ドメイン限定 | `allow_domain("posts")` |
| allow_action($a) | アクション限定 | `allow_action("read")` |
| match_resource_prefix($p) | 階層リソース | `match_resource_prefix("posts:123")` |

## 発行フロー (例)

1. ユーザログイン → JWT(短期) + Refresh(JTI=`<session>_refresh_vN`) + Biscuit(比較的広め権限)
2. クライアントがサブプロセスや別サービスへ再委譲: 親 Biscuit に新 block を追加 (write → read 限定 等)
3. サービス側で Biscuit 検証: 署名 → caveat(check) 評価 → ポリシー判定

## デュアル認証モデル

`handlers::mod::openapi_json` で OpenAPI 生成後に各 operation へ security を挿入:

- 多数エンドポイント: `[ {"BearerAuth":[]}, {"BiscuitAuth":[]} ]` (OR 条件)
- 公開エンドポイント: security 無し
- Biscuit 専用: `/api/v1/search/reindex` → `[ {"BiscuitAuth":[]} ]`

## ポリシー / Block 例 (疑似 Datalog)

```datalog
// Authority facts
authority {
  user_id("550e...");
  role("editor");
  cap("posts","write","*");
}

// Attenuated block (下位サービス) - write を read に制限
block1 {
  cap("posts","read","*");
  check allow_action("read");
  check time_before(1736000000);
}
```

### Caveat 評価の考え方

Checks は「すべて満たされなければ失敗 (deny)」です。`allow_action("read")` を含む attenuated block は read 以外の action を暗黙拒否。将来的に `deny_*` スタイルを導入するより、肯定形 allow の積み上げでホワイトリスト化する方針です。

## Rust 検証コード断片 (例)

```rust
use biscuit_auth::{Biscuit, KeyPair, macros::authorizer};

fn verify(token_bytes: &[u8], root_public: &biscuit_auth::PublicKey) -> anyhow::Result<()> {
    let b = Biscuit::from(token_bytes, root_public)?; // 署名検証

    // ここでは単純な authorizer を構築 (本番ではドメインポリシーを別モジュール化)
    let mut a = authorizer!(r#"
      allow if cap($d, $a, $r), $d == "posts", $a == "read";
    "#);

    // トークン内の facts / checks を authorizer に供給
    a.add_token(&b)?;

  // (任意) リクエスト文脈 (対象リソースを fact で追加)
  a.add_fact("cap(\"posts\", \"read\", \"123\")")?;

    a.authorize()?; // 失敗時は Err
    Ok(())
}
```

ポイント:

- `add_token` が blocks / facts / checks を authorizer に取り込み
- `authorize()` で全 checks & allow ルール評価
- 追加のリクエストコンテキスト (対象リソースなど) を fact として注入

## セキュリティ戦略サマリ

- Refresh ローテーション: 盗難 refresh の再利用無効化 (version インクリメントで前バージョン JTI 破棄)
- Biscuit は寿命を長めに設定しても caveat / checks で厳格化
- 高リスク操作 (現状: 再インデックス) を Biscuit 専用化し、JWT から段階的に分離
- 重大インシデント時: root 公開鍵ローテ → 旧鍵 ID を失効リストへ

## 今後の拡張アイデア

- Block chain 長さ & caveat 数へ上限 → DOS/メモリ防止
- Revocation list (session + biscuit unique id) と短期キャッシュ (Redis) で高速参照
- ポリシーファイル外部化 & ホットリロード (feature flag)
- WebAuthn + Biscuit 多要素 (高価値トランザクション保護)
- Observability: 検証失敗原因 (which check failed) を構造化ログへ出力

---

改善案や追加したい制約/ファクトがあれば Issue へ。

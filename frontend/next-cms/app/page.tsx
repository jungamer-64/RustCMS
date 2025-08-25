export default function Page() {
  return (
    <main className="grid">
      <section className="card">
        <h2>ようこそ</h2>
        <p className="muted">Rust製CMSのAPIに接続するフロントエンドです。</p>
        <ul>
          <li>GET /api/v1/posts 一覧</li>
          <li>GET /api/v1/posts/:id 詳細</li>
        </ul>
        <p><a className="btn" href="/posts">投稿一覧を見る</a></p>
      </section>
      <section className="card">
        <h3>設定</h3>
        <p className="muted">NEXT_PUBLIC_API_BASE_URL: {process.env.NEXT_PUBLIC_API_BASE_URL}</p>
      </section>
    </main>
  )
}

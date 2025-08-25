import { fetchJson } from '@/lib/api'
import { PostResponse } from '@/lib/types'

export const dynamic = 'force-dynamic'

async function getPost(id: string) {
  return fetchJson<PostResponse>(`/posts/${id}`)
}

export default async function PostDetail({ params }: { params: { id: string } }) {
  try {
    const post = await getPost(params.id)
    return (
      <main className="grid">
        <article className="card" style={{ gridColumn: '1 / -1' }}>
          <h2>{post.title}</h2>
          <p className="muted">{new Date(post.created_at).toLocaleString()}</p>
          <div style={{ marginTop: 16, whiteSpace: 'pre-wrap', lineHeight: 1.7 }}>
            {post.content}
          </div>
        </article>
      </main>
    )
  } catch (e: any) {
    return (
      <main className="grid">
        <section className="card" style={{ gridColumn: '1 / -1' }}>
          <h2>投稿詳細</h2>
          <p className="muted">APIに接続できませんでした。</p>
          <pre style={{ whiteSpace: 'pre-wrap' }}>{String(e?.message || e)}</pre>
          <p className="muted">バックエンドが起動しているか、NEXT_PUBLIC_API_BASE_URL({process.env.NEXT_PUBLIC_API_BASE_URL}) の設定をご確認ください。</p>
        </section>
      </main>
    )
  }
}

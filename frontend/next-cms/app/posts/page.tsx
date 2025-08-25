import { fetchJson } from '@/lib/api'
import { PostsResponse } from '@/lib/types'
import { PostList } from '@/components/PostList'

export const dynamic = 'force-dynamic'

async function getPosts(page: number, limit: number) {
  const qs = new URLSearchParams({ page: String(page), limit: String(limit) })
  return fetchJson<PostsResponse>(`/posts?${qs.toString()}`)
}

export default async function PostsPage({ searchParams }: { searchParams: { page?: string, limit?: string } }) {
  const page = Number(searchParams.page || '1')
  const limit = Number(searchParams.limit || '10')
  try {
    const data = await getPosts(page, limit)
    return (
      <main className="grid">
        <section className="card">
          <h2>投稿一覧</h2>
          <PostList posts={data.posts} />
          <div style={{ display: 'flex', gap: 8, marginTop: 16 }}>
            {page > 1 && <a className="btn" href={`/posts?page=${page - 1}&limit=${limit}`}>前へ</a>}
            {page < data.total_pages && <a className="btn" href={`/posts?page=${page + 1}&limit=${limit}`}>次へ</a>}
          </div>
        </section>
        <aside className="card">
          <h3>ページ情報</h3>
          <p className="muted">全 {data.total} 件 / {data.total_pages} ページ</p>
        </aside>
      </main>
    )
  } catch (e: any) {
    return (
      <main className="grid">
        <section className="card" style={{ gridColumn: '1 / -1' }}>
          <h2>投稿一覧</h2>
          <p className="muted">APIに接続できませんでした。</p>
          <pre style={{ whiteSpace: 'pre-wrap' }}>{String(e?.message || e)}</pre>
          <p className="muted">バックエンドが起動しているか、NEXT_PUBLIC_API_BASE_URL({process.env.NEXT_PUBLIC_API_BASE_URL}) の設定をご確認ください。</p>
        </section>
      </main>
    )
  }
}

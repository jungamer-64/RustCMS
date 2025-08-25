import Link from 'next/link'
import { PostResponse } from '@/lib/types'

export function PostList({ posts }: { posts: PostResponse[] }) {
  if (!posts.length) return <p className="muted">投稿がありません。</p>
  return (
    <div className="list">
      {posts.map((p) => (
        <article className="card" key={p.id}>
          <h3 style={{ margin: 0 }}>
            <Link href={`/posts/${p.id}`}>{p.title}</Link>
          </h3>
          <p className="muted" style={{ marginTop: 8 }}>
            {new Date(p.created_at).toLocaleString()} · {p.tags?.join(', ') || 'no tags'}
          </p>
          <p style={{ marginTop: 8 }}>{p.excerpt || (p.content?.slice(0, 140) + '...')}</p>
        </article>
      ))}
    </div>
  )
}

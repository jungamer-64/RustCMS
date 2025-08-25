export type PostResponse = {
  id: string
  title: string
  content: string
  excerpt?: string | null
  status: string
  author_id: string
  tags: string[]
  metadata: Record<string, unknown>
  created_at: string
  updated_at: string
  published_at?: string | null
}

export type PostsResponse = {
  posts: PostResponse[]
  total: number
  page: number
  limit: number
  total_pages: number
}

export type ApiEnvelope<T> = {
  success: boolean
  data?: T
  message?: string | null
  error?: string | null
}

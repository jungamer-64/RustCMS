import { ApiEnvelope } from './types'

const INTERNAL_PROXY_PREFIX = '/api/proxy'

export function apiPath(path: string) {
  if (!path.startsWith('/')) path = '/' + path
  return `${INTERNAL_PROXY_PREFIX}${path}`
}

export async function fetchJson<T>(input: string, init?: RequestInit): Promise<T> {
  const res = await fetch(apiPath(input), {
    ...init,
    // Avoid caching in SSR to reflect latest posts
    cache: 'no-store',
    headers: {
      'Content-Type': 'application/json',
      ...(init?.headers || {}),
    },
  })

  if (!res.ok) {
    const text = await res.text()
    throw new Error(`API error ${res.status}: ${text}`)
  }

  // Some endpoints return plain JSON, some wrap with ApiResponse
  const json = await res.json() as T | ApiEnvelope<T>
  if ((json as ApiEnvelope<T>).success !== undefined) {
    const env = json as ApiEnvelope<T>
    if (env.success && env.data !== undefined) return env.data
    if (env.success && env.data === undefined) return undefined as unknown as T
    throw new Error(env.error || 'Unknown API error')
  }
  return json as T
}

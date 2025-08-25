import { NextRequest } from 'next/server'

const API_BASE = process.env.NEXT_PUBLIC_API_BASE_URL || 'http://localhost:3000/api/v1'

function buildTargetUrl(path: string[], req: NextRequest) {
  const url = new URL(API_BASE)
  const suffix = path.join('/')
  // Ensure single slash between base and suffix
  url.pathname = `${url.pathname.replace(/\/$/, '')}/${suffix}`
  // Copy query string
  req.nextUrl.searchParams.forEach((v, k) => url.searchParams.set(k, v))
  return url.toString()
}

async function proxy(method: string, req: NextRequest, context: { params: { path: string[] } }) {
  const target = buildTargetUrl(context.params.path || [], req)
  const headers = new Headers(req.headers)
  headers.delete('host')
  headers.delete('connection')

  const init: RequestInit = {
    method,
    headers,
    // @ts-expect-error body can be null for GET/HEAD
    body: ['GET', 'HEAD'].includes(method) ? null : await req.text(),
    // Avoid caching on server
    cache: 'no-store'
  }

  const res = await fetch(target, init)
  const resHeaders = new Headers(res.headers)
  // Normalize CORS headers for same-origin
  resHeaders.set('access-control-allow-origin', '*')
  const body = await res.arrayBuffer()
  return new Response(body, { status: res.status, headers: resHeaders })
}

export async function GET(req: NextRequest, context: { params: { path: string[] } }) {
  return proxy('GET', req, context)
}

export async function POST(req: NextRequest, context: { params: { path: string[] } }) {
  return proxy('POST', req, context)
}

export async function PUT(req: NextRequest, context: { params: { path: string[] } }) {
  return proxy('PUT', req, context)
}

export async function DELETE(req: NextRequest, context: { params: { path: string[] } }) {
  return proxy('DELETE', req, context)
}

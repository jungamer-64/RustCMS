import './globals.css'
import type { ReactNode } from 'react'

export const metadata = {
  title: 'Rust CMS Frontend',
  description: 'Frontend for Rust CMS API',
}

export default function RootLayout({ children }: { children: ReactNode }) {
  return (
    <html lang="ja">
      <body>
        <div className="container">
          <header className="header">
            <h1>Rust CMS</h1>
            <nav className="nav">
              <a href="/">Home</a>
              <a href="/posts">Posts</a>
            </nav>
          </header>
          {children}
        </div>
      </body>
    </html>
  )
}

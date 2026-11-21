import { Link } from 'react-router-dom'

export function NotFoundPage() {
  return (
    <section className="page">
      <h1>Page not found</h1>
      <p>The requested admin page does not exist.</p>
      <Link className="text-link" to="/">
        Return to dashboard
      </Link>
    </section>
  )
}

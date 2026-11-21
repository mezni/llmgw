import { useEffect, useState } from 'react'
import { api } from '../api/client'
import type { DashboardSummary } from '../api/client'
import { StatusBadge } from '../components/common/StatusBadge'
import { formatCurrency, formatNumber, formatPercent } from '../lib/format'

export function DashboardPage() {
  const [summary, setSummary] = useState<DashboardSummary | null>(null)
  const [error, setError] = useState<string | null>(null)
  const [reloadKey, setReloadKey] = useState(0)

  useEffect(() => {
    let active = true
    setError(null)
    api
      .getDashboard()
      .then((value) => {
        if (active) setSummary(value)
      })
      .catch(() => {
        if (active) setError('The local mock data could not be loaded.')
      })
    return () => {
      active = false
    }
  }, [reloadKey])

  if (error) {
    return (
      <section className="page">
        <h1>Dashboard</h1>
        <div className="error-state" role="alert">
          <p>{error}</p>
          <button type="button" onClick={() => setReloadKey((value) => value + 1)}>
            Retry
          </button>
        </div>
      </section>
    )
  }

  if (!summary) {
    return (
      <section className="page">
        <h1>Dashboard</h1>
        <div className="skeleton-grid" aria-label="Loading dashboard metrics">
          <span />
          <span />
          <span />
          <span />
        </div>
      </section>
    )
  }

  return (
    <section className="page">
      <header className="page-header">
        <div>
          <h1>Dashboard</h1>
          <p>Usage for the last 24 hours from local mock data.</p>
        </div>
        <StatusBadge status="ok" />
      </header>
      <div className="metric-grid">
        <article className="metric-card">
          <p>Requests</p>
          <strong>{formatNumber(summary.requests24h)}</strong>
        </article>
        <article className="metric-card">
          <p>Tokens</p>
          <strong>{formatNumber(summary.tokens24h)}</strong>
        </article>
        <article className="metric-card">
          <p>Cost</p>
          <strong>{formatCurrency(summary.cost24h)}</strong>
        </article>
        <article className="metric-card">
          <p>P95 latency</p>
          <strong>{formatNumber(summary.p95LatencyMs)} ms</strong>
        </article>
        <article className="metric-card">
          <p>Error rate</p>
          <strong>{formatPercent(summary.errorRate)}</strong>
        </article>
      </div>
      <section className="panel">
        <h2>Daily requests</h2>
        <table>
          <caption className="sr-only">Requests by day</caption>
          <thead>
            <tr>
              <th scope="col">Day</th>
              <th scope="col">Requests</th>
            </tr>
          </thead>
          <tbody>
            {summary.series.map((point) => (
              <tr key={point.label}>
                <th scope="row">{point.label}</th>
                <td className="numeric">{formatNumber(point.requests)}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </section>
    </section>
  )
}

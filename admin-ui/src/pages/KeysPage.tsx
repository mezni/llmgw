import { useEffect, useState } from 'react'
import { api } from '../api/client'
import type { ApiKey } from '../api/client'
import { StatusBadge } from '../components/common/StatusBadge'

export function KeysPage() {
  const [keys, setKeys] = useState<ApiKey[] | null>(null)

  useEffect(() => {
    let active = true
    api.listKeys().then((value) => {
      if (active) setKeys(value)
    })
    return () => {
      active = false
    }
  }, [])

  return (
    <section className="page">
      <header className="page-header">
        <div>
          <h1>Keys</h1>
          <p>Display prefixes only; no secret values are available in the scaffold.</p>
        </div>
      </header>
      <section className="panel">
        <table>
          <caption className="sr-only">API keys</caption>
          <thead>
            <tr>
              <th scope="col">Prefix</th>
              <th scope="col">Project</th>
              <th scope="col">Created</th>
              <th scope="col">Status</th>
            </tr>
          </thead>
          <tbody>
            {keys?.map((key) => (
              <tr key={key.id}>
                <th scope="row" className="numeric">
                  {key.prefix}
                </th>
                <td>{key.project}</td>
                <td className="numeric">{key.created}</td>
                <td>
                  <StatusBadge status={key.status} />
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </section>
    </section>
  )
}

import { useEffect, useState } from 'react'
import { api } from '../api/client'
import type { Project } from '../api/client'
import { StatusBadge } from '../components/common/StatusBadge'
import { formatCurrency } from '../lib/format'

export function ProjectsPage() {
  const [projects, setProjects] = useState<Project[] | null>(null)

  useEffect(() => {
    let active = true
    api.listProjects().then((value) => {
      if (active) setProjects(value)
    })
    return () => {
      active = false
    }
  }, [])

  return (
    <section className="page">
      <header className="page-header">
        <div>
          <h1>Projects</h1>
          <p>Budgets and monthly spend from local mock data.</p>
        </div>
      </header>
      <section className="panel">
        <table>
          <caption className="sr-only">Projects</caption>
          <thead>
            <tr>
              <th scope="col">Project</th>
              <th scope="col">Status</th>
              <th scope="col">Monthly spend</th>
              <th scope="col">Budget</th>
            </tr>
          </thead>
          <tbody>
            {projects?.map((project) => (
              <tr key={project.id}>
                <th scope="row">{project.name}</th>
                <td>
                  <StatusBadge status={project.status} />
                </td>
                <td className="numeric">{formatCurrency(project.monthlySpend)}</td>
                <td className="numeric">{formatCurrency(project.budget)}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </section>
    </section>
  )
}

import { render, screen } from '@testing-library/react'
import { MemoryRouter } from 'react-router-dom'
import { describe, expect, it } from 'vitest'
import { App } from './App'

const renderAt = (path: string) =>
  render(
    <MemoryRouter initialEntries={[path]}>
      <App />
    </MemoryRouter>,
  )

describe('admin UI scaffold', () => {
  it('renders the dashboard from local mock data', async () => {
    renderAt('/')

    expect(screen.getByRole('heading', { name: 'Dashboard', level: 1 })).toBeInTheDocument()
    expect(screen.getByRole('navigation', { name: 'Admin navigation' })).toBeInTheDocument()
    expect(await screen.findByText('12,840')).toBeInTheDocument()
    expect(screen.getByText('Local mock data')).toBeInTheDocument()
  })

  it('renders key prefixes without secret values', async () => {
    renderAt('/keys')

    expect(await screen.findByRole('heading', { name: 'Keys' })).toBeInTheDocument()
    expect(await screen.findByText('gw-mock01')).toBeInTheDocument()
    expect(screen.queryByText(/sk-/)).not.toBeInTheDocument()
  })

  it('renders a not found page for unknown routes', async () => {
    renderAt('/missing')

    expect(await screen.findByRole('heading', { name: 'Page not found' })).toBeInTheDocument()
  })
})

import { NavLink, Outlet } from 'react-router-dom'

const navigation = [
  { to: '/', label: 'Dashboard', end: true },
  { to: '/projects', label: 'Projects', end: false },
  { to: '/keys', label: 'Keys', end: false },
]

export function AppShell() {
  return (
    <div className="app-shell">
      <a className="skip-link" href="#main-content">
        Skip to content
      </a>
      <aside className="sidebar" aria-label="Admin navigation">
        <div className="brand">
          <span className="brand-mark" aria-hidden="true">
            GW
          </span>
          <span>LLM Gateway</span>
        </div>
        <nav aria-label="Admin navigation">
          <ul className="nav-list">
            {navigation.map((item) => (
              <li key={item.to}>
                <NavLink
                  to={item.to}
                  end={item.end}
                  className={({ isActive }) => (isActive ? 'nav-link is-active' : 'nav-link')}
                >
                  {item.label}
                </NavLink>
              </li>
            ))}
          </ul>
        </nav>
      </aside>
      <div className="workspace">
        <header className="topbar">
          <span className="topbar-title">Admin console</span>
          <span className="env-pill">Local mock data</span>
        </header>
        <main id="main-content" tabIndex={-1}>
          <Outlet />
        </main>
      </div>
    </div>
  )
}

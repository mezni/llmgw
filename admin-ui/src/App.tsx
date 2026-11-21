import { Route, Routes } from 'react-router-dom'
import { AppShell } from './components/layout/AppShell'
import { DashboardPage } from './pages/DashboardPage'
import { KeysPage } from './pages/KeysPage'
import { NotFoundPage } from './pages/NotFoundPage'
import { ProjectsPage } from './pages/ProjectsPage'

export function App() {
  return (
    <Routes>
      <Route element={<AppShell />}>
        <Route index element={<DashboardPage />} />
        <Route path="projects" element={<ProjectsPage />} />
        <Route path="keys" element={<KeysPage />} />
        <Route path="*" element={<NotFoundPage />} />
      </Route>
    </Routes>
  )
}

import type { AdminApi, ApiKey, DashboardSummary, Project } from './client'

const summary: DashboardSummary = {
  requests24h: 12840,
  tokens24h: 3420000,
  cost24h: 18.42,
  p95LatencyMs: 842,
  errorRate: 0.004,
  series: [
    { label: 'Mon', requests: 1680 },
    { label: 'Tue', requests: 1920 },
    { label: 'Wed', requests: 1540 },
    { label: 'Thu', requests: 2110 },
    { label: 'Fri', requests: 2380 },
    { label: 'Sat', requests: 1120 },
    { label: 'Sun', requests: 2090 },
  ],
}

const projects: Project[] = [
  { id: 'proj_demo_a', name: 'Demo workspace', status: 'active', monthlySpend: 12.4, budget: 50 },
  { id: 'proj_demo_b', name: 'Evaluation lab', status: 'active', monthlySpend: 4.18, budget: 25 },
  { id: 'proj_demo_c', name: 'Archive', status: 'paused', monthlySpend: 1.84, budget: 10 },
]

const keys: ApiKey[] = [
  { id: 'key_demo_1', prefix: 'gw-mock01', project: 'Demo workspace', created: '2026-11-02', status: 'active' },
  { id: 'key_demo_2', prefix: 'gw-mock02', project: 'Evaluation lab', created: '2026-11-08', status: 'active' },
  { id: 'key_demo_3', prefix: 'gw-mock03', project: 'Archive', created: '2026-10-14', status: 'revoked' },
]

const pause = () => new Promise<void>((resolve) => setTimeout(resolve, 10))

export const mockApi: AdminApi = {
  async getDashboard() {
    await pause()
    return { ...summary, series: [...summary.series] }
  },
  async listProjects() {
    await pause()
    return projects.map((project) => ({ ...project }))
  },
  async listKeys() {
    await pause()
    return keys.map((key) => ({ ...key }))
  },
}

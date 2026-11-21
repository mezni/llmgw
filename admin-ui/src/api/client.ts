import { mockApi } from './mock'

export type RequestStatus = 'ok' | 'error' | 'cancelled'

export interface DashboardPoint {
  label: string
  requests: number
}

export interface DashboardSummary {
  requests24h: number
  tokens24h: number
  cost24h: number
  p95LatencyMs: number
  errorRate: number
  series: DashboardPoint[]
}

export interface Project {
  id: string
  name: string
  status: 'active' | 'paused'
  monthlySpend: number
  budget: number
}

export interface ApiKey {
  id: string
  prefix: string
  project: string
  created: string
  status: 'active' | 'revoked'
}

export interface AdminApi {
  getDashboard(): Promise<DashboardSummary>
  listProjects(): Promise<Project[]>
  listKeys(): Promise<ApiKey[]>
}

export const api: AdminApi = mockApi

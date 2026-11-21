interface StatusBadgeProps {
  status: 'active' | 'paused' | 'revoked' | 'ok' | 'error'
}

const labels: Record<StatusBadgeProps['status'], string> = {
  active: 'Active',
  paused: 'Paused',
  revoked: 'Revoked',
  ok: 'OK',
  error: 'Error',
}

export function StatusBadge({ status }: StatusBadgeProps) {
  return (
    <span className={`status-badge status-${status}`}>
      <span className="status-dot" aria-hidden="true" />
      {labels[status]}
    </span>
  )
}

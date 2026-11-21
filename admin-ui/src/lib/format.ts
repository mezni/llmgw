export const formatNumber = (value: number) => value.toLocaleString('en-US')

export const formatCurrency = (value: number) =>
  value.toLocaleString('en-US', { style: 'currency', currency: 'USD' })

export const formatPercent = (value: number) => `${(value * 100).toFixed(1)}%`

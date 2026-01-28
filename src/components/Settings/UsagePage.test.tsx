import { describe, it, expect, beforeEach, vi } from 'vitest'
import { render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { UsagePage } from './UsagePage'
import { mockInvoke, resetTauriMocks } from '@/test/tauri-mocks'

const mockMetrics = {
  cost_30d_cents: 1250,
  cost_7d_cents: 350,
  cost_24h_cents: 75,
  seconds_30d: 3600,
  seconds_7d: 900,
  seconds_24h: 120,
  requests_30d: 150,
  requests_7d: 42,
  requests_24h: 8,
  last_updated: new Date().toISOString(),
}

describe('UsagePage', () => {
  beforeEach(() => {
    resetTauriMocks()
  })

  it('shows admin key required message when key is not configured', async () => {
    mockInvoke('get_admin_key_status', { configured: false, masked_key: null })
    mockInvoke('get_cached_usage_metrics', null)

    render(<UsagePage />)

    await waitFor(() => {
      expect(screen.getByText('Admin API Key Required')).toBeInTheDocument()
    })

    expect(screen.getByText(/To view your OpenAI API usage metrics/)).toBeInTheDocument()
    expect(screen.getByText(/Go to/)).toBeInTheDocument()
  })

  it('shows usage page with refresh button when key is configured', async () => {
    mockInvoke('get_admin_key_status', { configured: true, masked_key: 'sk-...abc' })
    mockInvoke('get_cached_usage_metrics', null)

    render(<UsagePage />)

    await waitFor(() => {
      expect(screen.getByText('API Usage')).toBeInTheDocument()
    })

    expect(screen.getByRole('button', { name: /Refresh/i })).toBeInTheDocument()
  })

  it('displays metrics when cached data is available', async () => {
    mockInvoke('get_admin_key_status', { configured: true, masked_key: 'sk-...abc' })
    mockInvoke('get_cached_usage_metrics', mockMetrics)

    render(<UsagePage />)

    await waitFor(() => {
      expect(screen.getByText('Usage Statistics')).toBeInTheDocument()
    })

    // Check for costs
    expect(screen.getByText('$12.50')).toBeInTheDocument()
    expect(screen.getByText('$3.50')).toBeInTheDocument()
    expect(screen.getByText('$0.75')).toBeInTheDocument()

    // Check for audio durations
    expect(screen.getByText('1h 0m')).toBeInTheDocument()
    expect(screen.getByText('15m 0s')).toBeInTheDocument()
    expect(screen.getByText('2m 0s')).toBeInTheDocument()

    // Check for requests
    expect(screen.getByText('150')).toBeInTheDocument()
    expect(screen.getByText('42')).toBeInTheDocument()
    expect(screen.getByText('8')).toBeInTheDocument()
  })

  it('shows Load Metrics button when no cached data', async () => {
    mockInvoke('get_admin_key_status', { configured: true, masked_key: 'sk-...abc' })
    mockInvoke('get_cached_usage_metrics', null)

    render(<UsagePage />)

    await waitFor(() => {
      expect(screen.getByText('No usage data available.')).toBeInTheDocument()
    })

    expect(screen.getByRole('button', { name: 'Load Metrics' })).toBeInTheDocument()
  })

  it('fetches metrics when Load Metrics button is clicked', async () => {
    const user = userEvent.setup()
    mockInvoke('get_admin_key_status', { configured: true, masked_key: 'sk-...abc' })
    mockInvoke('get_cached_usage_metrics', null)

    let fetchCalled = false
    mockInvoke('fetch_usage_metrics', () => {
      fetchCalled = true
      return mockMetrics
    })

    render(<UsagePage />)

    await waitFor(() => {
      expect(screen.getByRole('button', { name: 'Load Metrics' })).toBeInTheDocument()
    })

    await user.click(screen.getByRole('button', { name: 'Load Metrics' }))

    await waitFor(() => {
      expect(fetchCalled).toBe(true)
    })

    await waitFor(() => {
      expect(screen.getByText('Usage Statistics')).toBeInTheDocument()
    })
  })

  it('fetches metrics when Refresh button is clicked', async () => {
    const user = userEvent.setup()
    mockInvoke('get_admin_key_status', { configured: true, masked_key: 'sk-...abc' })
    mockInvoke('get_cached_usage_metrics', mockMetrics)

    let fetchCalledWithForce = false
    mockInvoke('fetch_usage_metrics', (args: { forceRefresh: boolean }) => {
      if (args.forceRefresh) {
        fetchCalledWithForce = true
      }
      return mockMetrics
    })

    render(<UsagePage />)

    await waitFor(() => {
      expect(screen.getByRole('button', { name: /Refresh/i })).toBeInTheDocument()
    })

    await user.click(screen.getByRole('button', { name: /Refresh/i }))

    await waitFor(() => {
      expect(fetchCalledWithForce).toBe(true)
    })
  })

  it('shows error state when fetch fails', async () => {
    const user = userEvent.setup()
    mockInvoke('get_admin_key_status', { configured: true, masked_key: 'sk-...abc' })
    mockInvoke('get_cached_usage_metrics', null)
    mockInvoke('fetch_usage_metrics', () => {
      throw new Error('API rate limited')
    })

    render(<UsagePage />)

    await waitFor(() => {
      expect(screen.getByRole('button', { name: 'Load Metrics' })).toBeInTheDocument()
    })

    await user.click(screen.getByRole('button', { name: 'Load Metrics' }))

    await waitFor(() => {
      expect(screen.getByText('Failed to load metrics')).toBeInTheDocument()
    })

    expect(screen.getByText(/API rate limited/)).toBeInTheDocument()
    expect(screen.getByRole('button', { name: 'Retry' })).toBeInTheDocument()
  })

  it('displays budget progress card with metrics', async () => {
    mockInvoke('get_admin_key_status', { configured: true, masked_key: 'sk-...abc' })
    mockInvoke('get_cached_usage_metrics', mockMetrics)

    render(<UsagePage />)

    await waitFor(() => {
      expect(screen.getByText('Monthly Budget')).toBeInTheDocument()
    })

    // $12.50 of $25.00 = 50%
    expect(screen.getByText(/\$12\.50 of \$25\.00 used/)).toBeInTheDocument()
    expect(screen.getByText('50.0%')).toBeInTheDocument()
  })

  it('shows last updated time', async () => {
    mockInvoke('get_admin_key_status', { configured: true, masked_key: 'sk-...abc' })
    mockInvoke('get_cached_usage_metrics', mockMetrics)

    render(<UsagePage />)

    await waitFor(() => {
      expect(screen.getByText(/Last updated:/)).toBeInTheDocument()
    })
  })

  it('displays estimated words based on audio seconds', async () => {
    mockInvoke('get_admin_key_status', { configured: true, masked_key: 'sk-...abc' })
    mockInvoke('get_cached_usage_metrics', mockMetrics)

    render(<UsagePage />)

    await waitFor(() => {
      expect(screen.getByText('Est. Words')).toBeInTheDocument()
    })

    // 3600 seconds * 2.5 = 9000 words
    expect(screen.getByText('~9,000')).toBeInTheDocument()
  })
})

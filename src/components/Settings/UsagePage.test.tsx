import { describe, it, expect, beforeEach } from 'vitest'
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

const mockLocalSummary = {
  total_cycles: 10,
  successful_cycles: 8,
  failed_cycles: 2,
  avg_recording_duration_ms: 2500,
  avg_transcription_duration_ms: 850,
  avg_total_cycle_ms: 3350,
  last_error: null,
}

const mockCycleHistory = [
  {
    cycle_id: 'test-cycle-1',
    started_at: 1700000000,
    recording_duration_ms: 2000,
    audio_file_size_bytes: 48000,
    transcription_duration_ms: 800,
    transcript_length_chars: 120,
    total_cycle_ms: 2800,
    success: true,
    error_message: null,
  },
  {
    cycle_id: 'test-cycle-2',
    started_at: 1700000100,
    recording_duration_ms: 3000,
    audio_file_size_bytes: 72000,
    transcription_duration_ms: 900,
    transcript_length_chars: 0,
    total_cycle_ms: 3900,
    success: false,
    error_message: 'Network timeout',
  },
]

function setupLocalMetricsMocks() {
  mockInvoke('get_metrics_summary', mockLocalSummary)
  mockInvoke('get_metrics_history', mockCycleHistory)
}

describe('UsagePage', () => {
  beforeEach(() => {
    resetTauriMocks()
    setupLocalMetricsMocks()
  })

  // ========================================================================
  // Session Performance (always shown)
  // ========================================================================

  it('shows session performance card regardless of admin key', async () => {
    mockInvoke('get_admin_key_status', { configured: false, masked_key: null })
    mockInvoke('get_cached_usage_metrics', null)

    render(<UsagePage />)

    await waitFor(() => {
      expect(screen.getByText('Session Performance')).toBeInTheDocument()
    })
  })

  it('shows session performance metrics with data', async () => {
    mockInvoke('get_admin_key_status', { configured: false, masked_key: null })
    mockInvoke('get_cached_usage_metrics', null)

    render(<UsagePage />)

    await waitFor(() => {
      expect(screen.getByText('Total Cycles')).toBeInTheDocument()
    })

    expect(screen.getByText('10')).toBeInTheDocument()
    expect(screen.getByText('80%')).toBeInTheDocument() // 8/10 success
    expect(screen.getByText('2')).toBeInTheDocument() // failed
    expect(screen.getByText('2.5s')).toBeInTheDocument() // avg recording
    expect(screen.getByText('850ms')).toBeInTheDocument() // avg transcription
    expect(screen.getByText('3.4s')).toBeInTheDocument() // avg total
  })

  it('shows empty state when no cycles recorded', async () => {
    mockInvoke('get_admin_key_status', { configured: false, masked_key: null })
    mockInvoke('get_cached_usage_metrics', null)
    mockInvoke('get_metrics_summary', {
      ...mockLocalSummary,
      total_cycles: 0,
      successful_cycles: 0,
      failed_cycles: 0,
    })

    render(<UsagePage />)

    await waitFor(() => {
      expect(screen.getByText(/No transcription cycles recorded yet/)).toBeInTheDocument()
    })
  })

  it('shows last error in session performance card', async () => {
    mockInvoke('get_admin_key_status', { configured: false, masked_key: null })
    mockInvoke('get_cached_usage_metrics', null)
    mockInvoke('get_metrics_summary', {
      ...mockLocalSummary,
      last_error: {
        timestamp: 1700000200,
        error_type: 'transcription',
        message: 'API key expired',
        cycle_id: 'test-cycle-3',
      },
    })

    render(<UsagePage />)

    await waitFor(() => {
      expect(screen.getByText('Last Error')).toBeInTheDocument()
    })

    expect(screen.getByText('API key expired')).toBeInTheDocument()
  })

  // ========================================================================
  // Recent Cycles (always shown)
  // ========================================================================

  it('shows recent cycles card regardless of admin key', async () => {
    mockInvoke('get_admin_key_status', { configured: false, masked_key: null })
    mockInvoke('get_cached_usage_metrics', null)

    render(<UsagePage />)

    await waitFor(() => {
      expect(screen.getByText('Recent Cycles')).toBeInTheDocument()
    })
  })

  it('displays cycle history with correct data', async () => {
    mockInvoke('get_admin_key_status', { configured: false, masked_key: null })
    mockInvoke('get_cached_usage_metrics', null)

    render(<UsagePage />)

    await waitFor(() => {
      expect(screen.getByText('120')).toBeInTheDocument() // transcript chars
    })

    expect(screen.getByText('OK')).toBeInTheDocument()
    expect(screen.getByText('Fail')).toBeInTheDocument()
  })

  it('shows empty cycles message when no history', async () => {
    mockInvoke('get_admin_key_status', { configured: false, masked_key: null })
    mockInvoke('get_cached_usage_metrics', null)
    mockInvoke('get_metrics_history', [])

    render(<UsagePage />)

    await waitFor(() => {
      expect(screen.getByText('No cycles recorded yet.')).toBeInTheDocument()
    })
  })

  // ========================================================================
  // Admin Key Prompt (inline, not full-page blocker)
  // ========================================================================

  it('shows inline admin key prompt when key is not configured', async () => {
    mockInvoke('get_admin_key_status', { configured: false, masked_key: null })
    mockInvoke('get_cached_usage_metrics', null)

    render(<UsagePage />)

    await waitFor(() => {
      expect(screen.getByText('OpenAI Billing Metrics')).toBeInTheDocument()
    })

    expect(
      screen.getByText(/Configure an Admin API key in Settings/)
    ).toBeInTheDocument()

    // Should NOT show the full-page blocker
    expect(screen.queryByText('Admin API Key Required')).not.toBeInTheDocument()
  })

  // ========================================================================
  // OpenAI Billing (requires admin key)
  // ========================================================================

  it('shows OpenAI billing section with refresh button when key is configured', async () => {
    mockInvoke('get_admin_key_status', { configured: true, masked_key: 'sk-...abc' })
    mockInvoke('get_cached_usage_metrics', null)

    render(<UsagePage />)

    await waitFor(() => {
      expect(screen.getByText('OpenAI Billing')).toBeInTheDocument()
    })

    // The billing section's refresh button has visible "Refresh" text
    expect(screen.getByRole('button', { name: 'Refresh' })).toBeInTheDocument()
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
      expect(screen.getByRole('button', { name: 'Refresh' })).toBeInTheDocument()
    })

    await user.click(screen.getByRole('button', { name: 'Refresh' }))

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

  it('shows page title as Usage & Performance', async () => {
    mockInvoke('get_admin_key_status', { configured: false, masked_key: null })
    mockInvoke('get_cached_usage_metrics', null)

    render(<UsagePage />)

    await waitFor(() => {
      expect(screen.getByText('Usage & Performance')).toBeInTheDocument()
    })
  })
})

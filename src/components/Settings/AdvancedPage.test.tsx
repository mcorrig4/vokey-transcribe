import { describe, it, expect, beforeEach, vi } from 'vitest'
import { render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { AdvancedPage } from './AdvancedPage'
import { mockInvoke, mockInvokeMany, emitMockEvent, resetTauriMocks } from '@/test/tauri-mocks'

const mockHotkeyStatus = {
  active: true,
  device_count: 2,
  hotkey: 'Ctrl+Alt+Space',
  error: null,
}

const mockAudioStatus = {
  available: true,
  temp_dir: '/tmp/vokey',
  error: null,
}

const mockTranscriptionStatus = {
  api_key_configured: true,
  api_provider: 'OpenAI',
}

const mockMetrics = {
  total_cycles: 10,
  successful_cycles: 8,
  failed_cycles: 2,
  avg_recording_duration_ms: 3500,
  avg_transcription_duration_ms: 1200,
  avg_total_cycle_ms: 4700,
}

const mockKwinStatus = {
  is_wayland: true,
  is_kde: true,
  rules_applicable: true,
  rule_installed: false,
  config_path: '~/.config/kwinrc',
  error: null,
}

describe('AdvancedPage', () => {
  beforeEach(() => {
    resetTauriMocks()
    // Setup default mocks
    mockInvokeMany({
      get_hotkey_status: mockHotkeyStatus,
      get_audio_status: mockAudioStatus,
      get_transcription_status: mockTranscriptionStatus,
      get_metrics_summary: mockMetrics,
      get_kwin_status: mockKwinStatus,
    })
  })

  it('renders the advanced page with title', async () => {
    render(<AdvancedPage />)
    expect(screen.getByText('Advanced')).toBeInTheDocument()
    expect(screen.getByText('Developer tools, diagnostics, and system status.')).toBeInTheDocument()
  })

  it('displays current state as idle by default', async () => {
    render(<AdvancedPage />)

    await waitFor(() => {
      expect(screen.getByText('Current State')).toBeInTheDocument()
    })

    expect(screen.getByText('idle')).toBeInTheDocument()
  })

  it('updates state when state-update event is received', async () => {
    render(<AdvancedPage />)

    await waitFor(() => {
      expect(screen.getByText('idle')).toBeInTheDocument()
    })

    emitMockEvent('state-update', { status: 'recording', elapsedSecs: 5 })

    await waitFor(() => {
      expect(screen.getByText('recording')).toBeInTheDocument()
      expect(screen.getByText('5s')).toBeInTheDocument()
    })
  })

  it('displays hotkey status', async () => {
    render(<AdvancedPage />)

    await waitFor(() => {
      expect(screen.getByText('Hotkey')).toBeInTheDocument()
      expect(screen.getByText('Active')).toBeInTheDocument()
    })

    expect(screen.getByText(/Ctrl\+Alt\+Space/)).toBeInTheDocument()
    expect(screen.getByText(/2 devices/)).toBeInTheDocument()
  })

  it('displays audio status', async () => {
    render(<AdvancedPage />)

    await waitFor(() => {
      expect(screen.getByText('Audio')).toBeInTheDocument()
      expect(screen.getByText('Available')).toBeInTheDocument()
    })
  })

  it('displays transcription status', async () => {
    render(<AdvancedPage />)

    await waitFor(() => {
      expect(screen.getByText('Transcription')).toBeInTheDocument()
      expect(screen.getByText('Configured')).toBeInTheDocument()
      expect(screen.getByText('OpenAI')).toBeInTheDocument()
    })
  })

  it('displays session metrics', async () => {
    render(<AdvancedPage />)

    await waitFor(() => {
      expect(screen.getByText('Session Metrics')).toBeInTheDocument()
    })

    expect(screen.getByText('Total Cycles')).toBeInTheDocument()
    expect(screen.getByText('10')).toBeInTheDocument()
    expect(screen.getByText('Success Rate')).toBeInTheDocument()
    expect(screen.getByText('80%')).toBeInTheDocument()
    expect(screen.getByText('Avg Cycle Time')).toBeInTheDocument()
    expect(screen.getByText('4700ms')).toBeInTheDocument()
  })

  it('displays KWin rules section on Wayland + KDE', async () => {
    render(<AdvancedPage />)

    await waitFor(() => {
      expect(screen.getByText('Wayland HUD Setup')).toBeInTheDocument()
    })

    expect(screen.getByText('KWin Rule')).toBeInTheDocument()
    expect(screen.getByText('Not Installed')).toBeInTheDocument()
    expect(screen.getByRole('button', { name: 'Install' })).toBeInTheDocument()
  })

  it('hides KWin rules section when not applicable', async () => {
    mockInvoke('get_kwin_status', { ...mockKwinStatus, rules_applicable: false })

    render(<AdvancedPage />)

    await waitFor(() => {
      expect(screen.getByText('System Status')).toBeInTheDocument()
    })

    expect(screen.queryByText('Wayland HUD Setup')).not.toBeInTheDocument()
  })

  it('displays debug controls', async () => {
    render(<AdvancedPage />)

    await waitFor(() => {
      expect(screen.getByText('Debug Controls')).toBeInTheDocument()
    })

    expect(screen.getByRole('button', { name: /Start Recording/i })).toBeInTheDocument()
    expect(screen.getByRole('button', { name: /Stop Recording/i })).toBeInTheDocument()
    expect(screen.getByRole('button', { name: /Simulate Error/i })).toBeInTheDocument()
    expect(screen.getByRole('button', { name: /Cancel/i })).toBeInTheDocument()
  })

  it('calls simulate_record_start when Start Recording is clicked', async () => {
    const user = userEvent.setup()
    let invoked = false
    mockInvoke('simulate_record_start', () => { invoked = true })

    render(<AdvancedPage />)

    await waitFor(() => {
      expect(screen.getByRole('button', { name: /Start Recording/i })).toBeInTheDocument()
    })

    await user.click(screen.getByRole('button', { name: /Start Recording/i }))

    await waitFor(() => {
      expect(invoked).toBe(true)
    })
  })

  it('calls simulate_record_stop when Stop Recording is clicked', async () => {
    const user = userEvent.setup()
    let invoked = false
    mockInvoke('simulate_record_stop', () => { invoked = true })

    render(<AdvancedPage />)

    await waitFor(() => {
      expect(screen.getByRole('button', { name: /Stop Recording/i })).toBeInTheDocument()
    })

    await user.click(screen.getByRole('button', { name: /Stop Recording/i }))

    await waitFor(() => {
      expect(invoked).toBe(true)
    })
  })

  it('displays quick actions', async () => {
    render(<AdvancedPage />)

    await waitFor(() => {
      expect(screen.getByText('Quick Actions')).toBeInTheDocument()
    })

    expect(screen.getByRole('button', { name: /Open Logs Folder/i })).toBeInTheDocument()
    expect(screen.getByRole('button', { name: /Open Recordings/i })).toBeInTheDocument()
    expect(screen.getByRole('button', { name: /Copy Debug Info/i })).toBeInTheDocument()
  })

  it('calls open_logs_folder when Open Logs Folder is clicked', async () => {
    const user = userEvent.setup()
    let invoked = false
    mockInvoke('open_logs_folder', () => { invoked = true })

    render(<AdvancedPage />)

    await waitFor(() => {
      expect(screen.getByRole('button', { name: /Open Logs Folder/i })).toBeInTheDocument()
    })

    await user.click(screen.getByRole('button', { name: /Open Logs Folder/i }))

    await waitFor(() => {
      expect(invoked).toBe(true)
    })
  })

  it('copies debug info to clipboard and shows confirmation', async () => {
    const user = userEvent.setup()
    const mockWriteText = vi.fn().mockResolvedValue(undefined)
    Object.defineProperty(navigator, 'clipboard', {
      value: { writeText: mockWriteText },
      writable: true,
      configurable: true,
    })

    render(<AdvancedPage />)

    await waitFor(() => {
      expect(screen.getByRole('button', { name: /Copy Debug Info/i })).toBeInTheDocument()
    })

    await user.click(screen.getByRole('button', { name: /Copy Debug Info/i }))

    await waitFor(() => {
      expect(mockWriteText).toHaveBeenCalled()
      expect(screen.getByRole('button', { name: /Copied!/i })).toBeInTheDocument()
    })
  })

  it('installs KWin rule when Install button is clicked', async () => {
    const user = userEvent.setup()
    let installInvoked = false

    // Need to re-setup all mocks to ensure KWin section is shown
    mockInvokeMany({
      get_hotkey_status: mockHotkeyStatus,
      get_audio_status: mockAudioStatus,
      get_transcription_status: mockTranscriptionStatus,
      get_metrics_summary: mockMetrics,
      get_kwin_status: mockKwinStatus,
      install_kwin_rule: () => { installInvoked = true },
    })

    render(<AdvancedPage />)

    // Wait for the KWin section to appear
    await waitFor(() => {
      expect(screen.getByText('Wayland HUD Setup')).toBeInTheDocument()
    })

    // Find and click the Install button
    const installButton = screen.getByRole('button', { name: 'Install' })
    await user.click(installButton)

    await waitFor(() => {
      expect(installInvoked).toBe(true)
    })
  })

  it('shows Installed status when KWin rule is installed', async () => {
    mockInvoke('get_kwin_status', { ...mockKwinStatus, rule_installed: true })

    render(<AdvancedPage />)

    await waitFor(() => {
      expect(screen.getByText('Installed')).toBeInTheDocument()
    })

    expect(screen.getByRole('button', { name: 'Remove' })).toBeInTheDocument()
  })

  it('shows inactive hotkey status when hotkey is not active', async () => {
    mockInvoke('get_hotkey_status', { ...mockHotkeyStatus, active: false })

    render(<AdvancedPage />)

    await waitFor(() => {
      expect(screen.getByText('Inactive')).toBeInTheDocument()
    })
  })

  it('shows Not Configured when API key is not configured', async () => {
    mockInvoke('get_transcription_status', { ...mockTranscriptionStatus, api_key_configured: false })

    render(<AdvancedPage />)

    await waitFor(() => {
      expect(screen.getByText('Not Configured')).toBeInTheDocument()
    })
  })

  it('shows Unavailable when audio is not available', async () => {
    mockInvoke('get_audio_status', { ...mockAudioStatus, available: false })

    render(<AdvancedPage />)

    await waitFor(() => {
      expect(screen.getByText('Unavailable')).toBeInTheDocument()
    })
  })

  it('hides metrics when total_cycles is 0', async () => {
    mockInvoke('get_metrics_summary', { ...mockMetrics, total_cycles: 0 })

    render(<AdvancedPage />)

    await waitFor(() => {
      expect(screen.getByText('System Status')).toBeInTheDocument()
    })

    expect(screen.queryByText('Session Metrics')).not.toBeInTheDocument()
  })
})

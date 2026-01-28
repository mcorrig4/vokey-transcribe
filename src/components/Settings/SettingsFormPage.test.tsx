import { describe, it, expect, beforeEach, vi } from 'vitest'
import { render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { SettingsFormPage } from './SettingsFormPage'
import {
  mockInvoke,
  resetTauriMocks,
  type MockSettings,
} from '@/test/tauri-mocks'

// Mock AdminKeyInput to isolate tests
vi.mock('./AdminKeyInput', () => ({
  AdminKeyInput: () => <div data-testid="admin-key-input">AdminKeyInput</div>,
}))

const defaultSettings: MockSettings = {
  min_transcribe_ms: 500,
  short_clip_vad_enabled: true,
  vad_check_max_ms: 1500,
  vad_ignore_start_ms: 80,
  streaming_enabled: true,
}

describe('SettingsFormPage', () => {
  beforeEach(() => {
    resetTauriMocks()
  })

  it('shows loading state initially', () => {
    mockInvoke('get_settings', new Promise(() => {})) // Never resolves
    render(<SettingsFormPage />)
    expect(screen.getByText('Loading settings...')).toBeInTheDocument()
  })

  it('loads and displays settings', async () => {
    mockInvoke('get_settings', defaultSettings)
    render(<SettingsFormPage />)

    await waitFor(() => {
      expect(screen.getByText('Settings')).toBeInTheDocument()
    })

    expect(screen.getByText('Configure VoKey preferences and API credentials.')).toBeInTheDocument()
    expect(screen.getByLabelText('Enable Streaming')).toBeChecked()
  })

  it('shows error state on load failure', async () => {
    mockInvoke('get_settings', () => {
      throw new Error('Load failed')
    })
    render(<SettingsFormPage />)

    await waitFor(() => {
      expect(screen.getByText(/Failed to load settings/)).toBeInTheDocument()
    })

    expect(screen.getByRole('button', { name: 'Retry' })).toBeInTheDocument()
  })

  it('enables Save button when settings change', async () => {
    const user = userEvent.setup()
    mockInvoke('get_settings', defaultSettings)
    render(<SettingsFormPage />)

    await waitFor(() => {
      expect(screen.getByText('Settings')).toBeInTheDocument()
    })

    // Save button should be disabled initially
    const saveButton = screen.getByRole('button', { name: /Save Changes/i })
    expect(saveButton).toBeDisabled()

    // Toggle streaming
    const streamingSwitch = screen.getByLabelText('Enable Streaming')
    await user.click(streamingSwitch)

    // Save button should be enabled now
    expect(saveButton).toBeEnabled()
  })

  it('shows Reset button when settings change', async () => {
    const user = userEvent.setup()
    mockInvoke('get_settings', defaultSettings)
    render(<SettingsFormPage />)

    await waitFor(() => {
      expect(screen.getByText('Settings')).toBeInTheDocument()
    })

    // Reset button should not be visible initially
    expect(screen.queryByRole('button', { name: /Reset/i })).not.toBeInTheDocument()

    // Toggle streaming
    const streamingSwitch = screen.getByLabelText('Enable Streaming')
    await user.click(streamingSwitch)

    // Reset button should appear
    expect(screen.getByRole('button', { name: /Reset/i })).toBeInTheDocument()
  })

  it('calls set_settings when Save button is clicked', async () => {
    const user = userEvent.setup()
    mockInvoke('get_settings', defaultSettings)

    // Track if set_settings was called
    let setSettingsCalled = false
    mockInvoke('set_settings', () => {
      setSettingsCalled = true
      return undefined
    })

    render(<SettingsFormPage />)

    await waitFor(() => {
      expect(screen.getByText('Settings')).toBeInTheDocument()
    })

    // Toggle streaming to enable save button
    const streamingSwitch = screen.getByLabelText('Enable Streaming')
    await user.click(streamingSwitch)

    // Verify save button is enabled
    const saveButton = screen.getByRole('button', { name: /Save Changes/i })
    expect(saveButton).toBeEnabled()

    // Click save
    await user.click(saveButton)

    // Verify set_settings was called
    await waitFor(() => {
      expect(setSettingsCalled).toBe(true)
    })
  })

  it('resets settings when Reset button is clicked', async () => {
    const user = userEvent.setup()
    mockInvoke('get_settings', defaultSettings)
    render(<SettingsFormPage />)

    await waitFor(() => {
      expect(screen.getByText('Settings')).toBeInTheDocument()
    })

    // Toggle streaming off
    const streamingSwitch = screen.getByLabelText('Enable Streaming')
    expect(streamingSwitch).toBeChecked()
    await user.click(streamingSwitch)
    expect(streamingSwitch).not.toBeChecked()

    // Click reset
    const resetButton = screen.getByRole('button', { name: /Reset/i })
    await user.click(resetButton)

    // Should revert to original value
    expect(streamingSwitch).toBeChecked()
  })

  it('shows VAD settings when VAD is enabled', async () => {
    mockInvoke('get_settings', defaultSettings)
    render(<SettingsFormPage />)

    await waitFor(() => {
      expect(screen.getByText('Settings')).toBeInTheDocument()
    })

    // VAD settings should be visible
    expect(screen.getByLabelText('VAD Check Maximum Duration (ms)')).toBeInTheDocument()
    expect(screen.getByLabelText('VAD Ignore Start (ms)')).toBeInTheDocument()
  })

  it('hides VAD settings when VAD is disabled', async () => {
    const user = userEvent.setup()
    mockInvoke('get_settings', defaultSettings)
    render(<SettingsFormPage />)

    await waitFor(() => {
      expect(screen.getByText('Settings')).toBeInTheDocument()
    })

    // Disable VAD
    const vadSwitch = screen.getByLabelText('Short-clip Speech Detection (VAD)')
    await user.click(vadSwitch)

    // VAD settings should be hidden
    expect(screen.queryByLabelText('VAD Check Maximum Duration (ms)')).not.toBeInTheDocument()
    expect(screen.queryByLabelText('VAD Ignore Start (ms)')).not.toBeInTheDocument()
  })

  it('updates min transcribe duration', async () => {
    const user = userEvent.setup()
    mockInvoke('get_settings', defaultSettings)
    render(<SettingsFormPage />)

    await waitFor(() => {
      expect(screen.getByText('Settings')).toBeInTheDocument()
    })

    const input = screen.getByLabelText('Minimum Recording Duration (ms)')
    await user.clear(input)
    await user.type(input, '1000')

    // Save button should be enabled
    expect(screen.getByRole('button', { name: /Save Changes/i })).toBeEnabled()
  })

  it('includes AdminKeyInput component', async () => {
    mockInvoke('get_settings', defaultSettings)
    render(<SettingsFormPage />)

    await waitFor(() => {
      expect(screen.getByTestId('admin-key-input')).toBeInTheDocument()
    })
  })
})

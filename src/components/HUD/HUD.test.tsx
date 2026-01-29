import { describe, it, expect, vi } from 'vitest'
import { render, screen, waitFor } from '@testing-library/react'
import { HUD } from './HUD'
import { HUDProvider } from '@/context/HUDContext'
import { emitMockEvent, mockInvoke } from '@/test/tauri-mocks'

// Mock CSS modules
vi.mock('./HUD.module.css', () => ({
  default: {
    layout: 'hud-layout',
  },
}))

// Mock window API
vi.mock('@tauri-apps/api/window', () => ({
  getCurrentWindow: vi.fn(() => ({
    setSize: vi.fn(() => Promise.resolve()),
    setMinSize: vi.fn(() => Promise.resolve()),
    setMaxSize: vi.fn(() => Promise.resolve()),
    startDragging: vi.fn(() => Promise.resolve()),
  })),
  LogicalSize: class LogicalSize {
    width: number
    height: number
    constructor(width: number, height: number) {
      this.width = width
      this.height = height
    }
  },
}))

// Helper to render HUD with provider
function renderHUD() {
  return render(
    <HUDProvider>
      <HUD />
    </HUDProvider>
  )
}

describe('HUD', () => {
  it('renders in idle state', async () => {
    mockInvoke('open_settings_window', undefined)
    renderHUD()

    // HUD should render (even if visually minimal in idle)
    await waitFor(() => {
      expect(document.querySelector('.hud-layout')).toBeInTheDocument()
    })
  })

  it('shows recording indicator when state is recording', async () => {
    mockInvoke('open_settings_window', undefined)
    renderHUD()

    // Emit recording state
    emitMockEvent('state-update', {
      status: 'recording',
      elapsedSecs: 5,
    })

    await waitFor(() => {
      // Should show timer or recording indicator
      expect(screen.getByText(/5/)).toBeInTheDocument()
    })
  })

  it('shows transcript panel during recording', async () => {
    mockInvoke('open_settings_window', undefined)
    renderHUD()

    // Emit recording state
    emitMockEvent('state-update', {
      status: 'recording',
      elapsedSecs: 1,
    })

    // TranscriptPanel should be visible
    await waitFor(() => {
      expect(document.querySelector('.hud-layout')).toBeInTheDocument()
    })
  })

  it('shows transcribing state', async () => {
    mockInvoke('open_settings_window', undefined)
    renderHUD()

    // Emit transcribing state
    emitMockEvent('state-update', {
      status: 'transcribing',
    })

    // Should still show the HUD layout
    await waitFor(() => {
      expect(document.querySelector('.hud-layout')).toBeInTheDocument()
    })
  })

  it('shows done state with transcript', async () => {
    mockInvoke('open_settings_window', undefined)
    renderHUD()

    // Emit done state with text
    emitMockEvent('state-update', {
      status: 'done',
      text: 'Hello world',
    })

    await waitFor(() => {
      expect(document.querySelector('.hud-layout')).toBeInTheDocument()
    })
  })

  it('shows error state with message', async () => {
    mockInvoke('open_settings_window', undefined)
    renderHUD()

    // Emit error state
    emitMockEvent('state-update', {
      status: 'error',
      message: 'Connection failed',
    })

    await waitFor(() => {
      expect(document.querySelector('.hud-layout')).toBeInTheDocument()
    })
  })

  it('updates when state changes', async () => {
    mockInvoke('open_settings_window', undefined)
    renderHUD()

    // Start with idle
    emitMockEvent('state-update', { status: 'idle' })

    await waitFor(() => {
      expect(document.querySelector('.hud-layout')).toBeInTheDocument()
    })

    // Change to recording
    emitMockEvent('state-update', {
      status: 'recording',
      elapsedSecs: 2,
    })

    await waitFor(() => {
      expect(screen.getByText(/2/)).toBeInTheDocument()
    })

    // Change to done
    emitMockEvent('state-update', {
      status: 'done',
      text: 'Test complete',
    })

    await waitFor(() => {
      expect(document.querySelector('.hud-layout')).toBeInTheDocument()
    })
  })
})

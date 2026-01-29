import { describe, it, expect, beforeEach, vi } from 'vitest'
import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { AboutPage } from './AboutPage'

// Mock window.open
const mockOpen = vi.fn()
Object.defineProperty(window, 'open', {
  writable: true,
  value: mockOpen,
})

describe('AboutPage', () => {
  beforeEach(() => {
    mockOpen.mockClear()
  })

  it('renders the about page with title', () => {
    render(<AboutPage />)
    expect(screen.getByText('About VoKey')).toBeInTheDocument()
    expect(screen.getByText('Voice-to-text transcription via global hotkey.')).toBeInTheDocument()
  })

  it('displays app name and tagline', () => {
    render(<AboutPage />)
    expect(screen.getByText('VoKey Transcribe')).toBeInTheDocument()
    expect(screen.getByText('Press. Speak. Paste.')).toBeInTheDocument()
  })

  it('displays version number', async () => {
    render(<AboutPage />)
    // Wait for async version fetch to complete
    expect(await screen.findByText('0.2.0-dev')).toBeInTheDocument()
  })

  it('displays features list', () => {
    render(<AboutPage />)
    expect(screen.getByText('Features')).toBeInTheDocument()
    expect(screen.getByText('Global hotkey activation (Ctrl+Alt+Space)')).toBeInTheDocument()
    expect(screen.getByText('OpenAI Whisper transcription')).toBeInTheDocument()
    expect(screen.getByText('Real-time streaming transcription')).toBeInTheDocument()
    expect(screen.getByText('Automatic clipboard copy')).toBeInTheDocument()
    expect(screen.getByText('Linux/Wayland native support')).toBeInTheDocument()
    expect(screen.getByText('Voice activity detection (VAD)')).toBeInTheDocument()
  })

  it('displays external links section', () => {
    render(<AboutPage />)
    expect(screen.getByText('Links')).toBeInTheDocument()
    expect(screen.getByText('GitHub Repository')).toBeInTheDocument()
    expect(screen.getByText('Report an Issue')).toBeInTheDocument()
    expect(screen.getByText('Documentation')).toBeInTheDocument()
  })

  it('opens GitHub repository when clicked', async () => {
    const user = userEvent.setup()
    render(<AboutPage />)

    await user.click(screen.getByText('GitHub Repository'))

    expect(mockOpen).toHaveBeenCalledWith(
      'https://github.com/mcorrig4/vokey-transcribe',
      '_blank',
      'noopener,noreferrer'
    )
  })

  it('opens issues page when Report an Issue is clicked', async () => {
    const user = userEvent.setup()
    render(<AboutPage />)

    await user.click(screen.getByText('Report an Issue'))

    expect(mockOpen).toHaveBeenCalledWith(
      'https://github.com/mcorrig4/vokey-transcribe/issues',
      '_blank',
      'noopener,noreferrer'
    )
  })

  it('opens documentation when Documentation is clicked', async () => {
    const user = userEvent.setup()
    render(<AboutPage />)

    await user.click(screen.getByText('Documentation'))

    expect(mockOpen).toHaveBeenCalledWith(
      'https://github.com/mcorrig4/vokey-transcribe/blob/main/README.md',
      '_blank',
      'noopener,noreferrer'
    )
  })

  it('displays license information', () => {
    render(<AboutPage />)
    expect(screen.getByText('License & Credits')).toBeInTheDocument()
    expect(screen.getByText('AGPL-3.0-only')).toBeInTheDocument()
  })

  it('displays technology credits', () => {
    render(<AboutPage />)
    expect(screen.getByText('Built with')).toBeInTheDocument()
    expect(screen.getByText('Tauri')).toBeInTheDocument()
    expect(screen.getByText('React')).toBeInTheDocument()
    expect(screen.getByText('Rust')).toBeInTheDocument()
    expect(screen.getByText('Tailwind CSS')).toBeInTheDocument()
    expect(screen.getByText('OpenAI')).toBeInTheDocument()
  })

  it('displays made with love message', () => {
    render(<AboutPage />)
    expect(screen.getByText('Made with love for the Linux desktop')).toBeInTheDocument()
  })

  it('displays app icon emoji', () => {
    render(<AboutPage />)
    expect(screen.getByText('🎙️')).toBeInTheDocument()
  })
})

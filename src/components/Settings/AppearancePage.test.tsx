import { describe, it, expect, beforeEach, vi } from 'vitest'
import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { AppearancePage } from './AppearancePage'

// Mock matchMedia for theme detection
const mockMatchMedia = (matches: boolean) => {
  const listeners: ((e: MediaQueryListEvent) => void)[] = []
  return vi.fn().mockImplementation((query: string) => ({
    matches,
    media: query,
    onchange: null,
    addEventListener: vi.fn((_, handler) => listeners.push(handler)),
    removeEventListener: vi.fn(),
    dispatchEvent: vi.fn(),
    // For triggering theme changes in tests
    _triggerChange: (newMatches: boolean) => {
      listeners.forEach((l) => l({ matches: newMatches } as MediaQueryListEvent))
    },
  }))
}

describe('AppearancePage', () => {
  beforeEach(() => {
    // Default to light mode
    window.matchMedia = mockMatchMedia(false)
    // Reset document classes
    document.documentElement.classList.remove('light', 'dark')
  })

  it('renders appearance settings page', () => {
    render(<AppearancePage />)
    expect(screen.getByText('Appearance')).toBeInTheDocument()
    expect(screen.getByText('Customize the look and feel of VoKey.')).toBeInTheDocument()
  })

  it('displays theme options', () => {
    render(<AppearancePage />)
    expect(screen.getByText('Theme')).toBeInTheDocument()
    expect(screen.getByText('System')).toBeInTheDocument()
    expect(screen.getByText('Light')).toBeInTheDocument()
    expect(screen.getByText('Dark')).toBeInTheDocument()
  })

  it('displays HUD position options', () => {
    render(<AppearancePage />)
    expect(screen.getByText('HUD Position')).toBeInTheDocument()
    expect(screen.getByText('Top Left')).toBeInTheDocument()
    expect(screen.getByText('Top Right')).toBeInTheDocument()
    expect(screen.getByText('Bottom Left')).toBeInTheDocument()
    expect(screen.getByText('Bottom Right')).toBeInTheDocument()
  })

  it('displays animation settings', () => {
    render(<AppearancePage />)
    expect(screen.getByText('Animations')).toBeInTheDocument()
    expect(screen.getByLabelText('Enable Animations')).toBeInTheDocument()
    expect(screen.getByText('HUD Auto-hide Delay')).toBeInTheDocument()
  })

  it('displays auto-hide delay options', () => {
    render(<AppearancePage />)
    expect(screen.getByText('1s')).toBeInTheDocument()
    expect(screen.getByText('2s')).toBeInTheDocument()
    expect(screen.getByText('3s')).toBeInTheDocument()
    expect(screen.getByText('5s')).toBeInTheDocument()
    expect(screen.getByText('Never')).toBeInTheDocument()
  })

  it('selects theme when clicked', async () => {
    const user = userEvent.setup()
    render(<AppearancePage />)

    // System theme is selected by default
    const systemButton = screen.getByText('System').closest('button')
    expect(systemButton).toHaveClass('border-primary')

    // Click light theme
    const lightButton = screen.getByText('Light').closest('button')
    await user.click(lightButton!)

    // Light theme should be selected
    expect(lightButton).toHaveClass('border-primary')
    expect(systemButton).not.toHaveClass('border-primary')
    expect(screen.getByText('Using light theme.')).toBeInTheDocument()
  })

  it('applies dark theme to document', async () => {
    const user = userEvent.setup()
    render(<AppearancePage />)

    const darkButton = screen.getByText('Dark').closest('button')
    await user.click(darkButton!)

    expect(document.documentElement.classList.contains('dark')).toBe(true)
    expect(document.documentElement.classList.contains('light')).toBe(false)
  })

  it('applies light theme to document', async () => {
    const user = userEvent.setup()
    render(<AppearancePage />)

    const lightButton = screen.getByText('Light').closest('button')
    await user.click(lightButton!)

    expect(document.documentElement.classList.contains('light')).toBe(true)
    expect(document.documentElement.classList.contains('dark')).toBe(false)
  })

  it('shows system theme status when using system preference', () => {
    window.matchMedia = mockMatchMedia(false) // Light mode
    render(<AppearancePage />)

    expect(
      screen.getByText('Currently using light theme based on system preference.')
    ).toBeInTheDocument()
  })

  it('shows dark system preference status', () => {
    window.matchMedia = mockMatchMedia(true) // Dark mode
    render(<AppearancePage />)

    expect(
      screen.getByText('Currently using dark theme based on system preference.')
    ).toBeInTheDocument()
  })

  it('selects HUD position when clicked', async () => {
    const user = userEvent.setup()
    render(<AppearancePage />)

    // Top Left is default
    const topLeftButton = screen.getByText('Top Left').closest('button')
    expect(topLeftButton).toHaveClass('border-primary')

    // Click Top Right
    const topRightButton = screen.getByText('Top Right').closest('button')
    await user.click(topRightButton!)

    expect(topRightButton).toHaveClass('border-primary')
    expect(topLeftButton).not.toHaveClass('border-primary')
  })

  it('toggles animations on and off', async () => {
    const user = userEvent.setup()
    render(<AppearancePage />)

    const animationsSwitch = screen.getByLabelText('Enable Animations')
    expect(animationsSwitch).toBeChecked()

    await user.click(animationsSwitch)
    expect(animationsSwitch).not.toBeChecked()

    await user.click(animationsSwitch)
    expect(animationsSwitch).toBeChecked()
  })

  it('selects auto-hide delay', async () => {
    const user = userEvent.setup()
    render(<AppearancePage />)

    // 3s (3000ms) is default
    const defaultButton = screen.getByText('3s')
    expect(defaultButton).toHaveClass('bg-primary')

    // Click 5s
    const fiveSecButton = screen.getByText('5s')
    await user.click(fiveSecButton)

    expect(fiveSecButton).toHaveClass('bg-primary')
    expect(defaultButton).not.toHaveClass('bg-primary')
  })

  it('selects Never for auto-hide delay', async () => {
    const user = userEvent.setup()
    render(<AppearancePage />)

    const neverButton = screen.getByText('Never')
    await user.click(neverButton)

    expect(neverButton).toHaveClass('bg-primary')
  })

  it('shows note about KWin rules', () => {
    render(<AppearancePage />)
    expect(
      screen.getByText('Note: HUD position changes require KWin rule update on Wayland.')
    ).toBeInTheDocument()
  })

  it('shows real-time application note', () => {
    render(<AppearancePage />)
    expect(
      screen.getByText('Appearance settings are applied in real-time but not yet persisted across sessions.')
    ).toBeInTheDocument()
  })
})

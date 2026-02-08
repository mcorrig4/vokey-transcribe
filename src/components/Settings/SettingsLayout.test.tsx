import { describe, it, expect, beforeEach } from 'vitest'
import { render, screen, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { SettingsLayout } from './SettingsLayout'
import { resetTauriMocks } from '@/test/tauri-mocks'

describe('SettingsLayout', () => {
  beforeEach(() => {
    resetTauriMocks()
  })

  it('renders titlebar with correct title', () => {
    render(<SettingsLayout />)
    expect(screen.getByText('VoKey Settings')).toBeInTheDocument()
  })

  it('renders all navigation items', () => {
    render(<SettingsLayout />)

    expect(screen.getByTestId('settings-nav-usage')).toBeInTheDocument()
    expect(screen.getByTestId('settings-nav-settings')).toBeInTheDocument()
    expect(screen.getByTestId('settings-nav-appearance')).toBeInTheDocument()
    expect(screen.getByTestId('settings-nav-advanced')).toBeInTheDocument()
    expect(screen.getByTestId('settings-nav-about')).toBeInTheDocument()
  })

  it('shows nav labels when sidebar is expanded', () => {
    render(<SettingsLayout />)
    const sidebar = screen.getByTestId('settings-sidebar')

    expect(within(sidebar).getByText('Usage')).toBeInTheDocument()
    expect(within(sidebar).getByText('Settings')).toBeInTheDocument()
    expect(within(sidebar).getByText('Appearance')).toBeInTheDocument()
    expect(within(sidebar).getByText('Advanced')).toBeInTheDocument()
    expect(within(sidebar).getByText('About')).toBeInTheDocument()
  })

  it('defaults to Usage page active', () => {
    render(<SettingsLayout />)
    const usageNav = screen.getByTestId('settings-nav-usage')
    expect(usageNav.className).toContain('font-medium')
  })

  it('switches pages when nav items are clicked', async () => {
    const user = userEvent.setup()
    render(<SettingsLayout />)

    // Click About nav item
    await user.click(screen.getByTestId('settings-nav-about'))

    // About nav should be active
    const aboutNav = screen.getByTestId('settings-nav-about')
    expect(aboutNav.className).toContain('font-medium')

    // Usage nav should not be active
    const usageNav = screen.getByTestId('settings-nav-usage')
    expect(usageNav.className).not.toContain('font-medium')
  })

  it('collapses sidebar when collapse button is clicked', async () => {
    const user = userEvent.setup()
    render(<SettingsLayout />)

    // Collapse text visible
    const sidebar = screen.getByTestId('settings-sidebar')
    expect(within(sidebar).getByText('Collapse')).toBeInTheDocument()

    // Click collapse
    await user.click(screen.getByRole('button', { name: 'Collapse sidebar' }))

    // Labels should be hidden, sidebar narrowed
    expect(within(sidebar).queryByText('Usage')).not.toBeInTheDocument()
    expect(within(sidebar).queryByText('Collapse')).not.toBeInTheDocument()
    expect(sidebar.className).toContain('w-14')
  })

  it('expands sidebar when expand button is clicked', async () => {
    const user = userEvent.setup()
    render(<SettingsLayout />)

    // Collapse first
    await user.click(screen.getByRole('button', { name: 'Collapse sidebar' }))

    const sidebar = screen.getByTestId('settings-sidebar')
    expect(sidebar.className).toContain('w-14')

    // Expand
    await user.click(screen.getByRole('button', { name: 'Expand sidebar' }))

    expect(sidebar.className).toContain('w-48')
    expect(within(sidebar).getByText('Usage')).toBeInTheDocument()
  })

  it('shows tooltips on nav items when collapsed', async () => {
    const user = userEvent.setup()
    render(<SettingsLayout />)

    // Collapse sidebar
    await user.click(screen.getByRole('button', { name: 'Collapse sidebar' }))

    // Nav items should have title attributes for tooltips
    const usageNav = screen.getByTestId('settings-nav-usage')
    expect(usageNav).toHaveAttribute('title', 'Usage')
  })

  it('renders content area', () => {
    render(<SettingsLayout />)
    expect(screen.getByTestId('settings-content')).toBeInTheDocument()
  })

  it('renders children in content area', () => {
    render(
      <SettingsLayout>
        <div data-testid="custom-child">Custom content</div>
      </SettingsLayout>
    )

    const content = screen.getByTestId('settings-content')
    expect(within(content).getByTestId('custom-child')).toBeInTheDocument()
  })
})

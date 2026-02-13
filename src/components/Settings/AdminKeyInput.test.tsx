import { describe, it, expect, beforeEach, vi } from 'vitest'
import { render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { AdminKeyInput } from './AdminKeyInput'
import { openUrl } from '@tauri-apps/plugin-opener'
import {
  mockInvoke,
  resetTauriMocks,
} from '@/test/tauri-mocks'

describe('AdminKeyInput', () => {
  beforeEach(() => {
    resetTauriMocks()
    vi.mocked(openUrl).mockClear()
  })

  it('renders the card with title and description', async () => {
    mockInvoke('get_admin_key_status', { configured: false, masked_key: null })
    render(<AdminKeyInput />)

    expect(screen.getByText('OpenAI Admin API Key')).toBeInTheDocument()
    expect(screen.getByText(/Required for viewing usage metrics/)).toBeInTheDocument()
  })

  it('shows "Not configured" when no key is set', async () => {
    mockInvoke('get_admin_key_status', { configured: false, masked_key: null })
    render(<AdminKeyInput />)

    await waitFor(() => {
      expect(screen.getByText('Not configured')).toBeInTheDocument()
    })
  })

  it('shows "Configured" when key is set', async () => {
    mockInvoke('get_admin_key_status', { configured: true, masked_key: 'sk-admin-...abc' })
    render(<AdminKeyInput />)

    await waitFor(() => {
      expect(screen.getByText('Configured')).toBeInTheDocument()
    })
  })

  it('shows input field when no key is configured', async () => {
    mockInvoke('get_admin_key_status', { configured: false, masked_key: null })
    render(<AdminKeyInput />)

    await waitFor(() => {
      expect(screen.getByPlaceholderText('sk-admin-...')).toBeInTheDocument()
    })
  })

  it('shows Change Key and Remove Key buttons when key is configured', async () => {
    mockInvoke('get_admin_key_status', { configured: true, masked_key: 'sk-admin-...abc' })
    render(<AdminKeyInput />)

    await waitFor(() => {
      expect(screen.getByRole('button', { name: 'Change Key' })).toBeInTheDocument()
      expect(screen.getByRole('button', { name: 'Remove Key' })).toBeInTheDocument()
    })
  })

  it('shows input field when Change Key is clicked', async () => {
    const user = userEvent.setup()
    mockInvoke('get_admin_key_status', { configured: true, masked_key: 'sk-admin-...abc' })
    render(<AdminKeyInput />)

    await waitFor(() => {
      expect(screen.getByRole('button', { name: 'Change Key' })).toBeInTheDocument()
    })

    await user.click(screen.getByRole('button', { name: 'Change Key' }))

    expect(screen.getByPlaceholderText('sk-admin-...')).toBeInTheDocument()
    expect(screen.getByRole('button', { name: 'Cancel' })).toBeInTheDocument()
  })

  it('hides input and resets state when Cancel is clicked', async () => {
    const user = userEvent.setup()
    mockInvoke('get_admin_key_status', { configured: true, masked_key: 'sk-admin-...abc' })
    render(<AdminKeyInput />)

    await waitFor(() => {
      expect(screen.getByRole('button', { name: 'Change Key' })).toBeInTheDocument()
    })

    await user.click(screen.getByRole('button', { name: 'Change Key' }))
    expect(screen.getByPlaceholderText('sk-admin-...')).toBeInTheDocument()

    await user.click(screen.getByRole('button', { name: 'Cancel' }))
    expect(screen.queryByPlaceholderText('sk-admin-...')).not.toBeInTheDocument()
  })

  it('toggles masked key visibility', async () => {
    const user = userEvent.setup()
    mockInvoke('get_admin_key_status', { configured: true, masked_key: 'sk-admin-...abc' })
    render(<AdminKeyInput />)

    await waitFor(() => {
      expect(screen.getByText('Configured')).toBeInTheDocument()
    })

    // Initially masked
    expect(screen.getByText('••••••••••••')).toBeInTheDocument()

    // Click show
    await user.click(screen.getByRole('button', { name: 'Show key' }))
    expect(screen.getByText('sk-admin-...abc')).toBeInTheDocument()

    // Click hide
    await user.click(screen.getByRole('button', { name: 'Hide key' }))
    expect(screen.getByText('••••••••••••')).toBeInTheDocument()
  })

  it('disables Save button when validation is not valid', async () => {
    mockInvoke('get_admin_key_status', { configured: false, masked_key: null })
    render(<AdminKeyInput />)

    await waitFor(() => {
      expect(screen.getByPlaceholderText('sk-admin-...')).toBeInTheDocument()
    })

    expect(screen.getByRole('button', { name: 'Save' })).toBeDisabled()
  })

  it('opens OpenAI Dashboard link via openUrl', async () => {
    const user = userEvent.setup()
    mockInvoke('get_admin_key_status', { configured: false, masked_key: null })
    render(<AdminKeyInput />)

    await user.click(screen.getByText('Get Admin API Key from OpenAI Dashboard'))

    expect(openUrl).toHaveBeenCalledWith(
      'https://platform.openai.com/settings/organization/admin-keys'
    )
  })

  it('displays help text about required permissions', () => {
    mockInvoke('get_admin_key_status', { configured: false, masked_key: null })
    render(<AdminKeyInput />)

    expect(screen.getByText(/Create an Admin API key with "Usage: Read" permission/)).toBeInTheDocument()
  })

  it('calls remove key and reloads status', async () => {
    const user = userEvent.setup()
    let removeKeyCalled = false
    mockInvoke('get_admin_key_status', (_args: unknown) => {
      // After removal, return unconfigured
      if (removeKeyCalled) return { configured: false, masked_key: null }
      return { configured: true, masked_key: 'sk-admin-...abc' }
    })
    mockInvoke('set_admin_api_key', (_args: unknown) => {
      removeKeyCalled = true
      return undefined
    })

    render(<AdminKeyInput />)

    await waitFor(() => {
      expect(screen.getByRole('button', { name: 'Remove Key' })).toBeInTheDocument()
    })

    await user.click(screen.getByRole('button', { name: 'Remove Key' }))

    await waitFor(() => {
      expect(screen.getByText('Not configured')).toBeInTheDocument()
    })
  })

  it('accepts text input in the key field', async () => {
    const user = userEvent.setup()
    mockInvoke('get_admin_key_status', { configured: false, masked_key: null })
    render(<AdminKeyInput />)

    await waitFor(() => {
      expect(screen.getByPlaceholderText('sk-admin-...')).toBeInTheDocument()
    })

    const input = screen.getByPlaceholderText('sk-admin-...')
    await user.type(input, 'sk-admin-test')

    expect(input).toHaveValue('sk-admin-test')
  })
})

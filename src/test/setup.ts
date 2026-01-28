import '@testing-library/jest-dom/vitest'
import { vi, beforeAll, afterAll, beforeEach } from 'vitest'
import { initMocks, registerEventListener, setupDefaultTauriMocks } from './tauri-mocks'

// Create the mock invoke function
const mockInvokeFn = vi.fn()

// Mock Tauri APIs
vi.mock('@tauri-apps/api/core', () => ({
  invoke: mockInvokeFn,
}))

vi.mock('@tauri-apps/api/event', () => ({
  listen: vi.fn((event: string, handler: (event: unknown) => void) => {
    const unlisten = registerEventListener(event, handler)
    return Promise.resolve(unlisten)
  }),
  emit: vi.fn(),
}))

vi.mock('@tauri-apps/api/window', () => ({
  getCurrentWindow: vi.fn(() => ({
    close: vi.fn(),
    minimize: vi.fn(),
    maximize: vi.fn(),
    isMaximized: vi.fn(() => Promise.resolve(false)),
    onCloseRequested: vi.fn(() => Promise.resolve(() => {})),
  })),
}))

// Mock tauri-controls
vi.mock('tauri-controls', () => ({
  WindowControls: () => null,
  WindowTitlebar: ({ children }: { children: React.ReactNode }) => children,
}))

// Mock clipboard API
Object.assign(navigator, {
  clipboard: {
    writeText: vi.fn(() => Promise.resolve()),
    readText: vi.fn(() => Promise.resolve('')),
  },
})

// Initialize tauri mocks before all tests
beforeAll(() => {
  initMocks(mockInvokeFn)
})

// Setup default mocks before each test
beforeEach(() => {
  setupDefaultTauriMocks()
})

// Suppress console.error for expected errors in tests
const originalError = console.error
beforeAll(() => {
  console.error = (...args: unknown[]) => {
    const message = args[0]?.toString() || ''
    if (
      message.includes('Warning: ReactDOM.render is no longer supported') ||
      message.includes('act(...)') ||
      message.includes('Warning: An update to')
    ) {
      return
    }
    originalError.call(console, ...args)
  }
})

afterAll(() => {
  console.error = originalError
})

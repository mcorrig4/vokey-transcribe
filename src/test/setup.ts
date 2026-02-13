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

// Create reusable mock window object that can be accessed by tests
const mockWindowInstance = {
  close: vi.fn(() => Promise.resolve()),
  minimize: vi.fn(() => Promise.resolve()),
  maximize: vi.fn(() => Promise.resolve()),
  isMaximized: vi.fn(() => Promise.resolve(false)),
  onCloseRequested: vi.fn(() => Promise.resolve(() => {})),
  setMinSize: vi.fn(() => Promise.resolve()),
  setMaxSize: vi.fn(() => Promise.resolve()),
  setSize: vi.fn(() => Promise.resolve()),
  startDragging: vi.fn(() => Promise.resolve()),
  show: vi.fn(() => Promise.resolve()),
}

vi.mock('@tauri-apps/api/window', () => ({
  getCurrentWindow: vi.fn(() => mockWindowInstance),
  LogicalSize: class LogicalSize {
    width: number
    height: number
    constructor(width: number, height: number) {
      this.width = width
      this.height = height
    }
  },
}))

vi.mock('@tauri-apps/api/app', () => ({
  getVersion: vi.fn(() => Promise.resolve('0.2.0-dev')),
}))

vi.mock('@tauri-apps/plugin-opener', () => ({
  openUrl: vi.fn(() => Promise.resolve()),
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

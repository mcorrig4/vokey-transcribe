import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import App from './App'
import './styles/index.css'

// Determine which window to render based on URL parameter
const params = new URLSearchParams(window.location.search)
const windowType = params.get('window')

// Initialize app with conditional loading
async function init() {
  let RootComponent: React.ComponentType = App

  // Only load globals.css (Tailwind + shadcn theme) for Settings/Debug windows
  // HUD window needs transparent background without Tailwind's base styles
  if (windowType === 'settings' || windowType === 'debug') {
    // Dynamic import for Settings to avoid loading globals.css for HUD
    const [{ default: Settings }] = await Promise.all([
      import('./Settings'),
      import('./styles/globals.css'),
    ])
    RootComponent = Settings
  }

  createRoot(document.getElementById('root')!).render(
    <StrictMode>
      <RootComponent />
    </StrictMode>,
  )
}

init()

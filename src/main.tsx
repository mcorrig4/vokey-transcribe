import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'

// Determine which window to render based on URL parameter
const params = new URLSearchParams(window.location.search)
const windowType = params.get('window')

// Initialize app with conditional loading
// HUD and Settings load separate CSS to avoid cascade layer conflicts:
// - index.css (HUD): unlayered reset for transparent frameless window
// - globals.css (Settings): Tailwind v4 with @layer-based utilities
async function init() {
  let RootComponent: React.ComponentType

  if (windowType === 'settings' || windowType === 'debug') {
    const [{ default: Settings }] = await Promise.all([
      import('./Settings'),
      import('./styles/globals.css'),
    ])
    RootComponent = Settings
  } else {
    const [{ default: App }] = await Promise.all([
      import('./App'),
      import('./styles/index.css'),
    ])
    RootComponent = App
  }

  createRoot(document.getElementById('root')!).render(
    <StrictMode>
      <RootComponent />
    </StrictMode>,
  )
}

init().catch(err => {
  console.error('Failed to initialize app:', err)
  const root = document.getElementById('root')
  if (root) {
    root.innerHTML = `<pre style="color:red;padding:1em;font-family:monospace">${err}</pre>`
  }
})

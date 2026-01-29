import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import App from './App'
import Settings from './Settings'
import './styles/index.css'
import './styles/globals.css'

// Determine which window to render based on URL parameter
const params = new URLSearchParams(window.location.search)
const windowType = params.get('window')

// Set data attribute for window-specific styling (HUD needs transparent background)
document.documentElement.dataset.window = windowType || 'hud'

// Component map - explicit relationship between window type and component
const componentMap: Record<string, React.ComponentType> = {
  settings: Settings,
  debug: Settings, // Legacy debug window now uses Settings UI
}

const RootComponent = (windowType && componentMap[windowType]) || App

createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <RootComponent />
  </StrictMode>,
)

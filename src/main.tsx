import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import App from './App'
import Settings from './Settings'
import './styles/index.css'
import './styles/globals.css'

// Determine which window to render based on URL parameter
const params = new URLSearchParams(window.location.search)
const windowType = params.get('window')

// Component map - explicit relationship between window type and component
const componentMap: Record<string, React.ComponentType> = {
  settings: Settings,
  debug: Settings, // Legacy debug window now uses Settings UI
}

const RootComponent = (windowType && componentMap[windowType]) || App

// Set window type data attribute on html element for conditional CSS
// HUD window (no window type) needs transparent background
// Settings window needs opaque background
const htmlElement = document.documentElement
if (windowType) {
  htmlElement.setAttribute('data-window-type', windowType)
} else {
  htmlElement.setAttribute('data-window-type', 'hud')
}

createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <RootComponent />
  </StrictMode>,
)

import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import App from './App'
import Settings from './Settings'
import './styles/index.css'
import './styles/globals.css'

// Determine which window to render based on URL parameter
const params = new URLSearchParams(window.location.search)
const windowType = params.get('window')

function getRootComponent() {
  switch (windowType) {
    case 'settings':
    case 'debug': // Legacy debug window now uses Settings UI
      return Settings
    default:
      return App
  }
}

const RootComponent = getRootComponent()

createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <RootComponent />
  </StrictMode>,
)

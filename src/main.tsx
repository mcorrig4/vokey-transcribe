import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import App from './App'
import Debug from './Debug'
import './styles/index.css'

// Determine which window to render based on URL parameter
const params = new URLSearchParams(window.location.search)
const windowType = params.get('window')

const RootComponent = windowType === 'debug' ? Debug : App

createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <RootComponent />
  </StrictMode>,
)

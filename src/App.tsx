import { HUDProvider } from './context/HUDContext'
import { HUD } from './components/HUD'
import './styles/index.css'

function App() {
  return (
    <HUDProvider>
      <HUD />
    </HUDProvider>
  )
}

export default App

import { HUDProvider } from './context/HUDContext'
import { HUD } from './components/HUD'

function App() {
  return (
    <HUDProvider>
      <HUD />
    </HUDProvider>
  )
}

export default App

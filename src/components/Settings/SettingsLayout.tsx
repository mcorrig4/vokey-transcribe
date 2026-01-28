import { useState } from 'react'
import { TitleBar, Separator } from '@/components/ui'
import { cn } from '@/lib/utils'
import {
  BarChart3,
  Settings,
  Wrench,
  Info,
  ChevronLeft,
  ChevronRight
} from 'lucide-react'

// Navigation items
const navItems = [
  { id: 'usage', label: 'Usage', icon: BarChart3 },
  { id: 'settings', label: 'Settings', icon: Settings },
  { id: 'advanced', label: 'Advanced', icon: Wrench },
  { id: 'about', label: 'About', icon: Info },
] as const

type NavItem = typeof navItems[number]
type PageId = NavItem['id']

interface SettingsLayoutProps {
  children?: React.ReactNode
}

interface NavItemProps {
  item: NavItem
  isActive: boolean
  isCollapsed: boolean
  onClick: () => void
}

function NavItemButton({ item, isActive, isCollapsed, onClick }: NavItemProps) {
  const Icon = item.icon
  return (
    <button
      onClick={onClick}
      className={cn(
        "flex items-center gap-3 w-full px-3 py-2 rounded-md text-sm transition-colors",
        "hover:bg-accent hover:text-accent-foreground",
        isActive && "bg-accent text-accent-foreground font-medium",
        isCollapsed && "justify-center px-2"
      )}
      title={isCollapsed ? item.label : undefined}
      aria-label={item.label}
    >
      <Icon className="h-4 w-4 shrink-0" />
      {!isCollapsed && <span>{item.label}</span>}
    </button>
  )
}

export function SettingsLayout({ children }: SettingsLayoutProps) {
  const [activePage, setActivePage] = useState<PageId>('usage')
  const [isCollapsed, setIsCollapsed] = useState(false)

  return (
    <div className="h-screen flex flex-col bg-background text-foreground">
      <TitleBar title="VoKey Settings" />

      <div className="flex flex-1 overflow-hidden">
        {/* Sidebar */}
        <aside
          className={cn(
            "flex flex-col border-r border-border transition-all duration-200",
            isCollapsed ? "w-14" : "w-48"
          )}
        >
          <nav className="flex-1 p-2 space-y-1">
            {navItems.map((item) => (
              <NavItemButton
                key={item.id}
                item={item}
                isActive={activePage === item.id}
                isCollapsed={isCollapsed}
                onClick={() => setActivePage(item.id)}
              />
            ))}
          </nav>

          <Separator />

          {/* Collapse toggle */}
          <button
            onClick={() => setIsCollapsed(!isCollapsed)}
            className={cn(
              "flex items-center gap-2 p-2 text-muted-foreground hover:text-foreground transition-colors",
              isCollapsed && "justify-center"
            )}
            title={isCollapsed ? "Expand sidebar" : "Collapse sidebar"}
            aria-label={isCollapsed ? "Expand sidebar" : "Collapse sidebar"}
          >
            {isCollapsed ? (
              <ChevronRight className="h-4 w-4" />
            ) : (
              <>
                <ChevronLeft className="h-4 w-4" />
                <span className="text-xs">Collapse</span>
              </>
            )}
          </button>
        </aside>

        {/* Main content area */}
        <main className="flex-1 overflow-auto p-6">
          <SettingsContent page={activePage} />
          {children}
        </main>
      </div>
    </div>
  )
}

// Placeholder content components for each page - exhaustive switch for type safety
function SettingsContent({ page }: { page: PageId }): React.ReactNode {
  switch (page) {
    case 'usage':
      return <UsagePage />
    case 'settings':
      return <SettingsPage />
    case 'advanced':
      return <AdvancedPage />
    case 'about':
      return <AboutPage />
  }
}

function UsagePage() {
  return (
    <div className="space-y-4">
      <h2 className="text-2xl font-bold">API Usage</h2>
      <p className="text-muted-foreground">
        View your OpenAI API usage metrics and spending.
      </p>
      <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-3">
        <PlaceholderCard title="Current Month Spend" value="$0.00" />
        <PlaceholderCard title="Audio Transcribed" value="0 seconds" />
        <PlaceholderCard title="API Requests" value="0" />
      </div>
    </div>
  )
}

function SettingsPage() {
  return (
    <div className="space-y-4">
      <h2 className="text-2xl font-bold">Settings</h2>
      <p className="text-muted-foreground">
        Configure VoKey preferences and behavior.
      </p>
      <div className="text-sm text-muted-foreground italic">
        Settings configuration coming soon...
      </div>
    </div>
  )
}

function AdvancedPage() {
  return (
    <div className="space-y-4">
      <h2 className="text-2xl font-bold">Advanced</h2>
      <p className="text-muted-foreground">
        Advanced configuration and debugging options.
      </p>
      <div className="text-sm text-muted-foreground italic">
        Advanced options coming soon...
      </div>
    </div>
  )
}

function AboutPage() {
  return (
    <div className="space-y-4">
      <h2 className="text-2xl font-bold">About VoKey</h2>
      <p className="text-muted-foreground">
        Voice-to-text transcription via global hotkey.
      </p>
      <div className="space-y-2 text-sm">
        <p><strong>Version:</strong> 0.2.0-dev</p>
        <p><strong>License:</strong> AGPL-3.0-only</p>
        <p>
          <strong>Repository:</strong>{' '}
          <a
            href="https://github.com/mcorrig4/vokey-transcribe"
            className="text-primary hover:underline"
            target="_blank"
            rel="noopener noreferrer"
          >
            github.com/mcorrig4/vokey-transcribe
          </a>
        </p>
      </div>
    </div>
  )
}

function PlaceholderCard({ title, value }: { title: string; value: string }) {
  return (
    <div className="rounded-lg border border-border bg-card p-4">
      <p className="text-sm text-muted-foreground">{title}</p>
      <p className="text-2xl font-bold mt-1">{value}</p>
    </div>
  )
}

export default SettingsLayout

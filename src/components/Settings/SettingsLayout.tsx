import { useState } from 'react'
import { TitleBar, Separator } from '@/components/ui'
import { UsagePage } from './UsagePage'
import { SettingsFormPage } from './SettingsFormPage'
import { AppearancePage } from './AppearancePage'
import { AdvancedPage } from './AdvancedPage'
import { AboutPage } from './AboutPage'
import { cn } from '@/lib/utils'
import {
  BarChart3,
  Settings,
  Palette,
  Wrench,
  Info,
  ChevronLeft,
  ChevronRight
} from 'lucide-react'

// Navigation items
const navItems = [
  { id: 'usage', label: 'Usage', icon: BarChart3 },
  { id: 'settings', label: 'Settings', icon: Settings },
  { id: 'appearance', label: 'Appearance', icon: Palette },
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
        "flex items-center gap-3 w-full px-3 py-2 rounded-md text-sm transition-colors text-[var(--color-sidebar-foreground)]",
        "hover:bg-[var(--color-sidebar-accent)] hover:text-accent-foreground",
        isActive && "bg-[var(--color-sidebar-accent)] text-[var(--color-sidebar-accent-foreground)] font-medium",
        isCollapsed && "justify-center px-2"
      )}
      title={isCollapsed ? item.label : undefined}
      aria-label={item.label}
      data-testid={`settings-nav-${item.id}`}
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
            "flex flex-col border-r border-[var(--color-sidebar-border)] bg-[var(--color-sidebar-background)] transition-all duration-200",
            isCollapsed ? "w-14" : "w-48"
          )}
          data-testid="settings-sidebar"
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
              "flex items-center gap-2 p-2 text-[var(--color-sidebar-foreground)] hover:text-[var(--color-sidebar-accent-foreground)] transition-colors",
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
        <main className="flex-1 overflow-auto p-6 [&>*]:max-w-4xl" data-testid="settings-content">
          <SettingsContent page={activePage} />
          {children}
        </main>
      </div>
    </div>
  )
}

// Page component map - ensures explicit relationship between page ID and component
const pageMap: Record<PageId, React.ReactNode> = {
  usage: <UsagePage />,
  settings: <SettingsFormPage />,
  appearance: <AppearancePage />,
  advanced: <AdvancedPage />,
  about: <AboutPage />,
}

function SettingsContent({ page }: { page: PageId }): React.ReactNode {
  return pageMap[page] ?? null
}

export default SettingsLayout

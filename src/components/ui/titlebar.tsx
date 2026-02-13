import { getCurrentWindow } from '@tauri-apps/api/window'
import { cn } from "@/lib/utils"

interface TitleBarProps {
  title?: string
  className?: string
  children?: React.ReactNode
}

export function TitleBar({ title = "VoKey Settings", className, children }: TitleBarProps) {
  const appWindow = getCurrentWindow()

  return (
    <div
      className={cn(
        "h-10 bg-[var(--color-sidebar-background)] border-b border-border select-none flex items-center",
        className
      )}
    >
      <div
        className="flex items-center gap-2 px-3 flex-1"
        data-tauri-drag-region
      >
        {children || (
          <span className="text-sm font-medium text-foreground" data-tauri-drag-region>
            {title}
          </span>
        )}
      </div>
      <div className="flex items-center h-full">
        <button
          onClick={() => appWindow.minimize()}
          className="h-full px-3 text-muted-foreground hover:bg-accent hover:text-foreground transition-colors"
          aria-label="Minimize"
        >
          ─
        </button>
        <button
          onClick={() => appWindow.close()}
          className="h-full px-3 text-muted-foreground hover:bg-destructive hover:text-destructive-foreground transition-colors"
          aria-label="Close"
        >
          ✕
        </button>
      </div>
    </div>
  )
}

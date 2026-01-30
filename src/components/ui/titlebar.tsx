import { WindowTitlebar, WindowControls } from "tauri-controls"
import "tauri-controls/style.css"
import { cn } from "@/lib/utils"

interface TitleBarProps {
  title?: string
  className?: string
  children?: React.ReactNode
}

export function TitleBar({ title = "VoKey Settings", className, children }: TitleBarProps) {
  return (
    <WindowTitlebar
      className={cn(
        "h-10 bg-background border-b border-border select-none",
        className
      )}
      controlsOrder="right"
      windowControlsProps={{
        platform: "gnome",
        className: "ml-auto"
      }}
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
    </WindowTitlebar>
  )
}

export { WindowControls }

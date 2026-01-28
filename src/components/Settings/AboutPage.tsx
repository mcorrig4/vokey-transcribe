import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from '@/components/ui'
import { ExternalLink, Github, Bug, BookOpen, Heart } from 'lucide-react'

export function AboutPage() {
  const openExternal = (url: string) => {
    window.open(url, '_blank', 'noopener,noreferrer')
  }

  return (
    <div className="space-y-6">
      <div>
        <h2 className="text-2xl font-bold">About VoKey</h2>
        <p className="text-muted-foreground">
          Voice-to-text transcription via global hotkey.
        </p>
      </div>

      {/* App Info */}
      <Card>
        <CardHeader className="text-center">
          <div className="mx-auto mb-4">
            <div className="w-20 h-20 rounded-2xl bg-primary/10 flex items-center justify-center">
              <span className="text-4xl">🎙️</span>
            </div>
          </div>
          <CardTitle className="text-2xl">VoKey Transcribe</CardTitle>
          <CardDescription>
            Press. Speak. Paste.
          </CardDescription>
        </CardHeader>
        <CardContent className="text-center space-y-2">
          <div className="inline-flex items-center gap-2 px-3 py-1 rounded-full bg-muted text-sm">
            <span className="text-muted-foreground">Version</span>
            <span className="font-mono font-medium">0.2.0-dev</span>
          </div>
        </CardContent>
      </Card>

      {/* Features */}
      <Card>
        <CardHeader>
          <CardTitle>Features</CardTitle>
        </CardHeader>
        <CardContent>
          <ul className="space-y-2 text-sm">
            <li className="flex items-start gap-2">
              <span className="text-green-500">✓</span>
              <span>Global hotkey activation (Ctrl+Alt+Space)</span>
            </li>
            <li className="flex items-start gap-2">
              <span className="text-green-500">✓</span>
              <span>OpenAI Whisper transcription</span>
            </li>
            <li className="flex items-start gap-2">
              <span className="text-green-500">✓</span>
              <span>Real-time streaming transcription</span>
            </li>
            <li className="flex items-start gap-2">
              <span className="text-green-500">✓</span>
              <span>Automatic clipboard copy</span>
            </li>
            <li className="flex items-start gap-2">
              <span className="text-green-500">✓</span>
              <span>Linux/Wayland native support</span>
            </li>
            <li className="flex items-start gap-2">
              <span className="text-green-500">✓</span>
              <span>Voice activity detection (VAD)</span>
            </li>
          </ul>
        </CardContent>
      </Card>

      {/* Links */}
      <Card>
        <CardHeader>
          <CardTitle>Links</CardTitle>
        </CardHeader>
        <CardContent className="space-y-2">
          <button
            onClick={() => openExternal('https://github.com/mcorrig4/vokey-transcribe')}
            className="flex items-center gap-3 w-full p-3 rounded-lg hover:bg-accent transition-colors text-left"
          >
            <Github className="h-5 w-5" />
            <div className="flex-1">
              <p className="font-medium">GitHub Repository</p>
              <p className="text-sm text-muted-foreground">View source code and contribute</p>
            </div>
            <ExternalLink className="h-4 w-4 text-muted-foreground" />
          </button>

          <button
            onClick={() => openExternal('https://github.com/mcorrig4/vokey-transcribe/issues')}
            className="flex items-center gap-3 w-full p-3 rounded-lg hover:bg-accent transition-colors text-left"
          >
            <Bug className="h-5 w-5" />
            <div className="flex-1">
              <p className="font-medium">Report an Issue</p>
              <p className="text-sm text-muted-foreground">Found a bug? Let us know</p>
            </div>
            <ExternalLink className="h-4 w-4 text-muted-foreground" />
          </button>

          <button
            onClick={() => openExternal('https://github.com/mcorrig4/vokey-transcribe/blob/main/README.md')}
            className="flex items-center gap-3 w-full p-3 rounded-lg hover:bg-accent transition-colors text-left"
          >
            <BookOpen className="h-5 w-5" />
            <div className="flex-1">
              <p className="font-medium">Documentation</p>
              <p className="text-sm text-muted-foreground">Read the README and guides</p>
            </div>
            <ExternalLink className="h-4 w-4 text-muted-foreground" />
          </button>
        </CardContent>
      </Card>

      {/* License & Credits */}
      <Card>
        <CardHeader>
          <CardTitle>License & Credits</CardTitle>
        </CardHeader>
        <CardContent className="space-y-4">
          <div>
            <p className="text-sm text-muted-foreground">License</p>
            <p className="font-medium">AGPL-3.0-only</p>
          </div>
          <div>
            <p className="text-sm text-muted-foreground">Built with</p>
            <div className="flex flex-wrap gap-2 mt-1">
              {['Tauri', 'React', 'Rust', 'Tailwind CSS', 'OpenAI'].map((tech) => (
                <span
                  key={tech}
                  className="inline-flex items-center px-2 py-0.5 rounded bg-muted text-xs font-medium"
                >
                  {tech}
                </span>
              ))}
            </div>
          </div>
          <div className="flex items-center gap-2 text-sm text-muted-foreground pt-2">
            <Heart className="h-4 w-4 text-red-500" />
            <span>Made with love for the Linux desktop</span>
          </div>
        </CardContent>
      </Card>
    </div>
  )
}

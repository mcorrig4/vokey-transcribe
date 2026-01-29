import { useState, useEffect } from 'react'
import { invoke } from '@tauri-apps/api/core'
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
  Button,
  Progress,
  Skeleton,
} from '@/components/ui'
import { RefreshCw, AlertCircle, Key } from 'lucide-react'
import { cn } from '@/lib/utils'

interface UsageMetrics {
  cost_30d_cents: number
  cost_7d_cents: number
  cost_24h_cents: number
  seconds_30d: number
  seconds_7d: number
  seconds_24h: number
  requests_30d: number
  requests_7d: number
  requests_24h: number
  last_updated: string
}

interface AdminKeyStatus {
  configured: boolean
  masked_key: string | null
}

type LoadingState = 'idle' | 'loading' | 'success' | 'error'

// Formatting utilities
function formatCurrency(cents: number): string {
  return new Intl.NumberFormat('en-US', {
    style: 'currency',
    currency: 'USD',
  }).format(cents / 100)
}

function formatDuration(seconds: number): string {
  if (seconds < 60) return `${seconds}s`
  if (seconds < 3600) return `${Math.floor(seconds / 60)}m ${seconds % 60}s`
  const hours = Math.floor(seconds / 3600)
  const mins = Math.floor((seconds % 3600) / 60)
  return `${hours}h ${mins}m`
}

function formatNumber(num: number): string {
  return new Intl.NumberFormat('en-US').format(num)
}

function estimateWords(seconds: number): number {
  // Average speaking rate: ~2.5 words per second
  return Math.round(seconds * 2.5)
}

function formatRelativeTime(isoDate: string): string {
  const date = new Date(isoDate)
  const now = new Date()
  const diffMs = now.getTime() - date.getTime()
  const diffMins = Math.floor(diffMs / 60000)

  if (diffMins < 1) return 'just now'
  if (diffMins < 60) return `${diffMins} minute${diffMins === 1 ? '' : 's'} ago`
  const diffHours = Math.floor(diffMins / 60)
  if (diffHours < 24) return `${diffHours} hour${diffHours === 1 ? '' : 's'} ago`
  const diffDays = Math.floor(diffHours / 24)
  return `${diffDays} day${diffDays === 1 ? '' : 's'} ago`
}

export function UsagePage() {
  const [metrics, setMetrics] = useState<UsageMetrics | null>(null)
  const [keyStatus, setKeyStatus] = useState<AdminKeyStatus | null>(null)
  const [loadingState, setLoadingState] = useState<LoadingState>('idle')
  const [error, setError] = useState<string | null>(null)

  // Load admin key status on mount
  useEffect(() => {
    loadKeyStatus()
  }, [])

  // Load cached metrics on mount
  useEffect(() => {
    loadCachedMetrics()
  }, [])

  const loadKeyStatus = async () => {
    try {
      const status = await invoke<AdminKeyStatus>('get_admin_key_status')
      setKeyStatus(status)
    } catch (e) {
      console.error('Failed to get admin key status:', e)
    }
  }

  const loadCachedMetrics = async () => {
    try {
      const cached = await invoke<UsageMetrics | null>('get_cached_usage_metrics')
      if (cached) {
        setMetrics(cached)
        setLoadingState('success')
      }
    } catch (e) {
      console.error('Failed to get cached metrics:', e)
    }
  }

  const fetchMetrics = async (forceRefresh = false) => {
    setLoadingState('loading')
    setError(null)

    try {
      const data = await invoke<UsageMetrics>('fetch_usage_metrics', {
        forceRefresh,
      })
      setMetrics(data)
      setLoadingState('success')
    } catch (e) {
      setError(String(e))
      setLoadingState('error')
    }
  }

  // Not configured state
  if (keyStatus && !keyStatus.configured) {
    return (
      <div className="space-y-4">
        <h2 className="text-2xl font-bold">API Usage</h2>
        <Card>
          <CardContent className="flex flex-col items-center justify-center py-12 text-center">
            <Key className="h-12 w-12 text-muted-foreground mb-4" />
            <h3 className="text-lg font-semibold mb-2">Admin API Key Required</h3>
            <p className="text-muted-foreground max-w-md">
              To view your OpenAI API usage metrics, you need to configure an Admin API key
              with usage read permissions.
            </p>
            <p className="text-sm text-muted-foreground mt-4">
              Go to <strong>Settings</strong> to add your Admin API key.
            </p>
          </CardContent>
        </Card>
      </div>
    )
  }

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h2 className="text-2xl font-bold">API Usage</h2>
          <p className="text-muted-foreground">
            View your OpenAI API usage metrics and spending.
          </p>
        </div>
        <Button
          variant="outline"
          size="sm"
          onClick={() => fetchMetrics(true)}
          disabled={loadingState === 'loading'}
        >
          <RefreshCw
            className={cn("h-4 w-4 mr-2", loadingState === 'loading' && "animate-spin")}
          />
          Refresh
        </Button>
      </div>

      {/* Error state */}
      {loadingState === 'error' && error && (
        <Card className="border-destructive">
          <CardContent className="flex items-center gap-3 py-4">
            <AlertCircle className="h-5 w-5 text-destructive" />
            <div>
              <p className="font-medium text-destructive">Failed to load metrics</p>
              <p className="text-sm text-muted-foreground">{error}</p>
            </div>
            <Button
              variant="outline"
              size="sm"
              className="ml-auto"
              onClick={() => fetchMetrics(true)}
            >
              Retry
            </Button>
          </CardContent>
        </Card>
      )}

      {/* Metrics grid */}
      <Card>
        <CardHeader>
          <CardTitle>Usage Statistics</CardTitle>
          <CardDescription>
            Transcription costs and usage across different time periods.
          </CardDescription>
        </CardHeader>
        <CardContent>
          {loadingState === 'loading' && !metrics ? (
            <MetricsSkeleton />
          ) : metrics ? (
            <MetricsGrid metrics={metrics} />
          ) : (
            <div className="text-center py-8 text-muted-foreground">
              <p>No usage data available.</p>
              <Button
                variant="outline"
                size="sm"
                className="mt-4"
                onClick={() => fetchMetrics(false)}
              >
                Load Metrics
              </Button>
            </div>
          )}
        </CardContent>
      </Card>

      {/* Budget progress */}
      {metrics && <BudgetCard metrics={metrics} />}

      {/* Last updated */}
      {metrics && (
        <p className="text-sm text-muted-foreground text-center">
          Last updated: {formatRelativeTime(metrics.last_updated)}
        </p>
      )}
    </div>
  )
}

function MetricsGrid({ metrics }: { metrics: UsageMetrics }) {
  return (
    <div className="overflow-x-auto">
      <table className="w-full">
        <thead>
          <tr className="border-b border-border">
            <th className="text-left py-2 pr-4 font-medium text-muted-foreground">Metric</th>
            <th className="text-right py-2 px-4 font-medium">30-day</th>
            <th className="text-right py-2 px-4 font-medium">7-day</th>
            <th className="text-right py-2 px-4 font-medium">24-hour</th>
          </tr>
        </thead>
        <tbody>
          <tr className="border-b border-border">
            <td className="py-3 pr-4 text-muted-foreground">Cost</td>
            <td className="py-3 px-4 text-right font-mono">
              {formatCurrency(metrics.cost_30d_cents)}
            </td>
            <td className="py-3 px-4 text-right font-mono">
              {formatCurrency(metrics.cost_7d_cents)}
            </td>
            <td className="py-3 px-4 text-right font-mono">
              {formatCurrency(metrics.cost_24h_cents)}
            </td>
          </tr>
          <tr className="border-b border-border">
            <td className="py-3 pr-4 text-muted-foreground">Audio</td>
            <td className="py-3 px-4 text-right font-mono">
              {formatDuration(metrics.seconds_30d)}
            </td>
            <td className="py-3 px-4 text-right font-mono">
              {formatDuration(metrics.seconds_7d)}
            </td>
            <td className="py-3 px-4 text-right font-mono">
              {formatDuration(metrics.seconds_24h)}
            </td>
          </tr>
          <tr className="border-b border-border">
            <td className="py-3 pr-4 text-muted-foreground">Requests</td>
            <td className="py-3 px-4 text-right font-mono">
              {formatNumber(metrics.requests_30d)}
            </td>
            <td className="py-3 px-4 text-right font-mono">
              {formatNumber(metrics.requests_7d)}
            </td>
            <td className="py-3 px-4 text-right font-mono">
              {formatNumber(metrics.requests_24h)}
            </td>
          </tr>
          <tr>
            <td className="py-3 pr-4 text-muted-foreground">Est. Words</td>
            <td className="py-3 px-4 text-right font-mono text-muted-foreground">
              ~{formatNumber(estimateWords(metrics.seconds_30d))}
            </td>
            <td className="py-3 px-4 text-right font-mono text-muted-foreground">
              ~{formatNumber(estimateWords(metrics.seconds_7d))}
            </td>
            <td className="py-3 px-4 text-right font-mono text-muted-foreground">
              ~{formatNumber(estimateWords(metrics.seconds_24h))}
            </td>
          </tr>
        </tbody>
      </table>
    </div>
  )
}

function MetricsSkeleton() {
  return (
    <div className="space-y-3">
      <div className="flex justify-between">
        <Skeleton className="h-4 w-20" />
        <Skeleton className="h-4 w-16" />
        <Skeleton className="h-4 w-16" />
        <Skeleton className="h-4 w-16" />
      </div>
      {[1, 2, 3, 4].map((i) => (
        <div key={i} className="flex justify-between py-2">
          <Skeleton className="h-5 w-24" />
          <Skeleton className="h-5 w-20" />
          <Skeleton className="h-5 w-20" />
          <Skeleton className="h-5 w-20" />
        </div>
      ))}
    </div>
  )
}

function BudgetCard({ metrics }: { metrics: UsageMetrics }) {
  // Default budget: $25/month
  const monthlyBudgetCents = 2500
  const usagePercent = Math.min(100, (metrics.cost_30d_cents / monthlyBudgetCents) * 100)

  return (
    <Card>
      <CardHeader>
        <CardTitle className="text-lg">Monthly Budget</CardTitle>
      </CardHeader>
      <CardContent className="space-y-3">
        <Progress value={usagePercent} className="h-3" />
        <div className="flex justify-between text-sm">
          <span className="text-muted-foreground">
            {formatCurrency(metrics.cost_30d_cents)} of {formatCurrency(monthlyBudgetCents)} used
          </span>
          <span className={cn(
            "font-medium",
            usagePercent >= 90 ? "text-destructive" :
            usagePercent >= 75 ? "text-yellow-500" :
            "text-muted-foreground"
          )}>
            {usagePercent.toFixed(1)}%
          </span>
        </div>
      </CardContent>
    </Card>
  )
}

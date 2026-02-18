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
import { RefreshCw, AlertCircle, Key, Activity } from 'lucide-react'
import { cn } from '@/lib/utils'

// ============================================================================
// Types
// ============================================================================

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

interface MetricsSummary {
  total_cycles: number
  successful_cycles: number
  failed_cycles: number
  avg_recording_duration_ms: number
  avg_transcription_duration_ms: number
  avg_total_cycle_ms: number
  last_error: ErrorRecord | null
}

interface CycleMetrics {
  cycle_id: string
  started_at: number
  recording_duration_ms: number
  audio_file_size_bytes: number
  transcription_duration_ms: number
  transcript_length_chars: number
  total_cycle_ms: number
  success: boolean
  error_message: string | null
}

interface ErrorRecord {
  timestamp: number
  error_type: string
  message: string
  cycle_id: string | null
}

type LoadingState = 'idle' | 'loading' | 'success' | 'error'

// ============================================================================
// Formatting utilities
// ============================================================================

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

function formatMs(ms: number): string {
  if (ms < 1000) return `${ms}ms`
  return `${(ms / 1000).toFixed(1)}s`
}

function formatTimestamp(unixSeconds: number): string {
  const date = new Date(unixSeconds * 1000)
  return date.toLocaleTimeString('en-US', {
    hour: '2-digit',
    minute: '2-digit',
    second: '2-digit',
  })
}

// ============================================================================
// Main component
// ============================================================================

export function UsagePage() {
  const [metrics, setMetrics] = useState<UsageMetrics | null>(null)
  const [keyStatus, setKeyStatus] = useState<AdminKeyStatus | null>(null)
  const [loadingState, setLoadingState] = useState<LoadingState>('idle')
  const [error, setError] = useState<string | null>(null)
  const [localSummary, setLocalSummary] = useState<MetricsSummary | null>(null)
  const [cycleHistory, setCycleHistory] = useState<CycleMetrics[]>([])

  // Load admin key status on mount
  useEffect(() => {
    loadKeyStatus()
  }, [])

  // Load cached metrics on mount
  useEffect(() => {
    loadCachedMetrics()
  }, [])

  // Load local metrics on mount
  useEffect(() => {
    loadLocalMetrics()
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

  const loadLocalMetrics = async () => {
    try {
      const [summary, history] = await Promise.all([
        invoke<MetricsSummary>('get_metrics_summary'),
        invoke<CycleMetrics[]>('get_metrics_history'),
      ])
      setLocalSummary(summary)
      setCycleHistory(history)
    } catch (e) {
      console.error('Failed to load local metrics:', e)
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

  return (
    <div className="space-y-6">
      {/* Page header */}
      <div>
        <h2 className="text-2xl font-bold">Usage & Performance</h2>
        <p className="text-muted-foreground">
          Session performance metrics and OpenAI API usage.
        </p>
      </div>

      {/* Session Performance — always shown */}
      <SessionPerformanceCard summary={localSummary} onRefresh={loadLocalMetrics} />

      {/* Recent Cycles — always shown */}
      <CycleHistoryCard cycles={cycleHistory} />

      {/* OpenAI Billing — conditional on admin key */}
      {keyStatus?.configured ? (
        <>
          <div className="flex items-center justify-between">
            <h3 className="text-lg font-semibold">OpenAI Billing</h3>
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
        </>
      ) : keyStatus && (
        <AdminKeyPromptCard />
      )}
    </div>
  )
}

// ============================================================================
// Session Performance (local metrics — no admin key needed)
// ============================================================================

function SessionPerformanceCard({
  summary,
  onRefresh,
}: {
  summary: MetricsSummary | null
  onRefresh: () => void
}) {
  return (
    <Card>
      <CardHeader>
        <div className="flex items-center justify-between">
          <CardTitle className="flex items-center gap-2">
            <Activity className="h-5 w-5" />
            Session Performance
          </CardTitle>
          <Button variant="ghost" size="sm" onClick={onRefresh} aria-label="Refresh session metrics">
            <RefreshCw className="h-4 w-4" />
          </Button>
        </div>
        <CardDescription>
          Local recording and transcription metrics for this session.
        </CardDescription>
      </CardHeader>
      <CardContent>
        {!summary ? (
          <p className="text-muted-foreground text-center py-4">Loading metrics...</p>
        ) : summary.total_cycles === 0 ? (
          <p className="text-muted-foreground text-center py-4">
            No transcription cycles recorded yet. Use the hotkey to start recording.
          </p>
        ) : (
          <div className="grid grid-cols-2 md:grid-cols-3 gap-4">
            <MetricTile label="Total Cycles" value={String(summary.total_cycles)} />
            <MetricTile
              label="Success Rate"
              value={`${Math.round((summary.successful_cycles / summary.total_cycles) * 100)}%`}
              variant={
                summary.successful_cycles / summary.total_cycles >= 0.9
                  ? 'success'
                  : summary.successful_cycles / summary.total_cycles >= 0.7
                    ? 'warn'
                    : 'error'
              }
            />
            <MetricTile
              label="Failed"
              value={String(summary.failed_cycles)}
              variant={summary.failed_cycles > 0 ? 'error' : 'default'}
            />
            <MetricTile label="Avg Recording" value={formatMs(summary.avg_recording_duration_ms)} />
            <MetricTile label="Avg Transcription" value={formatMs(summary.avg_transcription_duration_ms)} />
            <MetricTile label="Avg Total Cycle" value={formatMs(summary.avg_total_cycle_ms)} />
          </div>
        )}
        {summary?.last_error && (
          <div className="mt-4 p-3 rounded-md bg-destructive/10 border border-destructive/20">
            <p className="text-sm font-medium text-destructive">Last Error</p>
            <p className="text-sm text-muted-foreground">{summary.last_error.message}</p>
          </div>
        )}
      </CardContent>
    </Card>
  )
}

function MetricTile({
  label,
  value,
  variant = 'default',
}: {
  label: string
  value: string
  variant?: 'default' | 'success' | 'warn' | 'error'
}) {
  return (
    <div>
      <p className="text-sm text-muted-foreground">{label}</p>
      <p
        className={cn(
          'text-lg font-semibold',
          variant === 'success' && 'text-green-500',
          variant === 'warn' && 'text-yellow-500',
          variant === 'error' && 'text-destructive',
        )}
      >
        {value}
      </p>
    </div>
  )
}

// ============================================================================
// Recent Cycles (local metrics — no admin key needed)
// ============================================================================

function CycleHistoryCard({ cycles }: { cycles: CycleMetrics[] }) {
  const recentCycles = cycles.slice(0, 10)

  return (
    <Card>
      <CardHeader>
        <CardTitle>Recent Cycles</CardTitle>
        <CardDescription>
          {recentCycles.length > 0
            ? `Last ${recentCycles.length} recording/transcription cycle${recentCycles.length === 1 ? '' : 's'}.`
            : 'Recording and transcription cycle history.'}
        </CardDescription>
      </CardHeader>
      <CardContent>
        {recentCycles.length === 0 ? (
          <p className="text-muted-foreground text-center py-4">No cycles recorded yet.</p>
        ) : (
          <div className="overflow-x-auto">
            <table className="w-full text-sm">
              <thead>
                <tr className="border-b border-border">
                  <th className="text-left py-2 pr-3 font-medium text-muted-foreground">Time</th>
                  <th className="text-right py-2 px-3 font-medium text-muted-foreground">Record</th>
                  <th className="text-right py-2 px-3 font-medium text-muted-foreground">Transcribe</th>
                  <th className="text-right py-2 px-3 font-medium text-muted-foreground">Total</th>
                  <th className="text-right py-2 px-3 font-medium text-muted-foreground">Chars</th>
                  <th className="text-center py-2 pl-3 font-medium text-muted-foreground">Status</th>
                </tr>
              </thead>
              <tbody>
                {recentCycles.map((cycle) => (
                  <tr key={cycle.cycle_id} className="border-b border-border last:border-0">
                    <td className="py-2 pr-3 text-muted-foreground">
                      {formatTimestamp(cycle.started_at)}
                    </td>
                    <td className="py-2 px-3 text-right font-mono">
                      {formatMs(cycle.recording_duration_ms)}
                    </td>
                    <td className="py-2 px-3 text-right font-mono">
                      {formatMs(cycle.transcription_duration_ms)}
                    </td>
                    <td className="py-2 px-3 text-right font-mono">
                      {formatMs(cycle.total_cycle_ms)}
                    </td>
                    <td className="py-2 px-3 text-right font-mono">
                      {cycle.transcript_length_chars}
                    </td>
                    <td className="py-2 pl-3 text-center">
                      {cycle.success ? (
                        <span className="inline-flex items-center px-1.5 py-0.5 rounded text-xs bg-green-500/20 text-green-500">
                          OK
                        </span>
                      ) : (
                        <span
                          className="inline-flex items-center px-1.5 py-0.5 rounded text-xs bg-destructive/20 text-destructive"
                          title={cycle.error_message ?? ''}
                        >
                          Fail
                        </span>
                      )}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}
      </CardContent>
    </Card>
  )
}

// ============================================================================
// Admin Key Prompt (inline — not a full-page blocker)
// ============================================================================

function AdminKeyPromptCard() {
  return (
    <Card className="border-dashed">
      <CardContent className="flex items-center gap-4 py-6">
        <Key className="h-8 w-8 text-muted-foreground shrink-0" />
        <div className="flex-1">
          <h3 className="font-medium">OpenAI Billing Metrics</h3>
          <p className="text-sm text-muted-foreground">
            Configure an Admin API key in Settings to view cost and API usage data from OpenAI.
          </p>
        </div>
      </CardContent>
    </Card>
  )
}

// ============================================================================
// OpenAI Billing components (unchanged)
// ============================================================================

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

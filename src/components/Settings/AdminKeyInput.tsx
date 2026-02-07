import { useState, useEffect, useCallback } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { Button, Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui'
import { Input } from '@/components/ui/input'
import { cn } from '@/lib/utils'
import { Eye, EyeOff, Check, X, Loader2, ExternalLink, Key } from 'lucide-react'

interface AdminKeyStatus {
  configured: boolean
  masked_key: string | null
}

type ValidationState = 'idle' | 'validating' | 'valid' | 'invalid'

export function AdminKeyInput() {
  const [status, setStatus] = useState<AdminKeyStatus>({ configured: false, masked_key: null })
  const [inputValue, setInputValue] = useState('')
  const [showInput, setShowInput] = useState(false)
  const [showKey, setShowKey] = useState(false)
  const [validationState, setValidationState] = useState<ValidationState>('idle')
  const [error, setError] = useState<string | null>(null)
  const [saving, setSaving] = useState(false)

  // Load status on mount
  useEffect(() => {
    loadStatus()
  }, [])

  const loadStatus = async () => {
    try {
      const s = await invoke<AdminKeyStatus>('get_admin_key_status')
      setStatus(s)
    } catch (e) {
      console.error('Failed to get admin key status:', e)
    }
  }

  const validateKey = useCallback(async (key: string) => {
    if (!key || key.length < 10) {
      setValidationState('idle')
      return
    }

    setValidationState('validating')
    setError(null)

    try {
      const valid = await invoke<boolean>('validate_admin_api_key', { key })
      setValidationState(valid ? 'valid' : 'invalid')
      if (!valid) {
        setError('Key is invalid or lacks usage read permissions')
      }
    } catch (e) {
      setValidationState('invalid')
      setError(String(e))
    }
  }, [])

  const saveKey = async () => {
    if (validationState !== 'valid') {
      return
    }

    setSaving(true)
    setError(null)

    try {
      await invoke('set_admin_api_key', { key: inputValue })
      await loadStatus()
      setInputValue('')
      setShowInput(false)
      setValidationState('idle')
    } catch (e) {
      setError(String(e))
    } finally {
      setSaving(false)
    }
  }

  const removeKey = async () => {
    setSaving(true)
    setError(null)

    try {
      await invoke('set_admin_api_key', { key: null })
      await loadStatus()
    } catch (e) {
      setError(String(e))
    } finally {
      setSaving(false)
    }
  }

  const handleInputChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const value = e.target.value
    setInputValue(value)
    setValidationState('idle')
    setError(null)
  }

  // Debounced validation effect
  useEffect(() => {
    if (inputValue.length >= 10) {
      const timer = setTimeout(() => validateKey(inputValue), 500)
      return () => clearTimeout(timer)
    }
  }, [inputValue, validateKey])

  return (
    <Card>
      <CardHeader>
        <CardTitle className="flex items-center gap-2">
          <Key className="h-5 w-5" />
          OpenAI Admin API Key
        </CardTitle>
        <CardDescription>
          Required for viewing usage metrics. Uses elevated permissions - stored securely in system keyring.
        </CardDescription>
      </CardHeader>
      <CardContent className="space-y-4">
        {/* Current status */}
        <div className="flex items-center gap-2">
          <span className="text-sm text-muted-foreground">Status:</span>
          {status.configured ? (
            <span className="flex items-center gap-1 text-sm text-green-500">
              <Check className="h-4 w-4" />
              Configured
            </span>
          ) : (
            <span className="flex items-center gap-1 text-sm text-yellow-500">
              <X className="h-4 w-4" />
              Not configured
            </span>
          )}
        </div>

        {/* Show masked key if configured */}
        {status.configured && status.masked_key && (
          <div className="flex items-center gap-2">
            <span className="text-sm text-muted-foreground">Key:</span>
            <code className="text-sm font-mono bg-muted px-2 py-1 rounded">
              {showKey ? status.masked_key : '••••••••••••'}
            </code>
            <Button
              variant="ghost"
              size="sm"
              onClick={() => setShowKey(!showKey)}
              aria-label={showKey ? 'Hide key' : 'Show key'}
            >
              {showKey ? <EyeOff className="h-4 w-4" /> : <Eye className="h-4 w-4" />}
            </Button>
          </div>
        )}

        {/* Input for new key */}
        {(showInput || !status.configured) && (
          <div className="space-y-2">
            <div className="flex gap-2">
              <div className="relative flex-1">
                <Input
                  type="password"
                  placeholder="sk-admin-..."
                  value={inputValue}
                  onChange={handleInputChange}
                  className={cn(
                    "pr-10",
                    validationState === 'valid' && "border-green-500",
                    validationState === 'invalid' && "border-red-500"
                  )}
                />
                <div className="absolute right-2 top-1/2 -translate-y-1/2">
                  {validationState === 'validating' && (
                    <Loader2 className="h-4 w-4 animate-spin text-muted-foreground" />
                  )}
                  {validationState === 'valid' && (
                    <Check className="h-4 w-4 text-green-500" />
                  )}
                  {validationState === 'invalid' && (
                    <X className="h-4 w-4 text-red-500" />
                  )}
                </div>
              </div>
              <Button
                onClick={saveKey}
                disabled={validationState !== 'valid' || saving}
              >
                {saving ? <Loader2 className="h-4 w-4 animate-spin" /> : 'Save'}
              </Button>
            </div>
            {error && (
              <p className="text-sm text-red-500">{error}</p>
            )}
          </div>
        )}

        {/* Actions */}
        <div className="flex items-center gap-2">
          {status.configured && !showInput && (
            <>
              <Button
                variant="outline"
                size="sm"
                onClick={() => setShowInput(true)}
              >
                Change Key
              </Button>
              <Button
                variant="outline"
                size="sm"
                onClick={removeKey}
                disabled={saving}
              >
                Remove Key
              </Button>
            </>
          )}
          {showInput && status.configured && (
            <Button
              variant="ghost"
              size="sm"
              onClick={() => {
                setShowInput(false)
                setInputValue('')
                setValidationState('idle')
                setError(null)
              }}
            >
              Cancel
            </Button>
          )}
        </div>

        {/* Help link */}
        <div className="pt-2 border-t border-border">
          <a
            href="https://platform.openai.com/settings/organization/admin-keys"
            target="_blank"
            rel="noopener noreferrer"
            className="inline-flex items-center gap-1 text-sm text-primary hover:underline"
          >
            Get Admin API Key from OpenAI Dashboard
            <ExternalLink className="h-3 w-3" />
          </a>
          <p className="mt-1 text-xs text-muted-foreground">
            Create an Admin API key with "Usage: Read" permission.
          </p>
        </div>
      </CardContent>
    </Card>
  )
}

import { useEffect, useState, useCallback } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { relaunch } from '@tauri-apps/plugin-process'
import styles from './SetupBanner.module.css'

interface KwinSetupNeeded {
  needs_setup: boolean
  already_prompted: boolean
  rules_applicable: boolean
}

type BannerState = 'hidden' | 'prompt' | 'installing' | 'success' | 'error'

/**
 * SetupBanner displays a one-time banner prompting users on Wayland/KDE
 * to install KWin rules for proper HUD behavior (positioning, always-on-top, no focus steal).
 */
export function SetupBanner() {
  const [bannerState, setBannerState] = useState<BannerState>('hidden')
  const [errorMsg, setErrorMsg] = useState<string | null>(null)

  // Check on mount if we need to show the banner
  useEffect(() => {
    async function checkSetup() {
      try {
        const result = await invoke<KwinSetupNeeded>('check_kwin_setup_needed')

        // Show banner if: setup is needed and user hasn't been prompted
        if (result.needs_setup && !result.already_prompted) {
          setBannerState('prompt')
        }
      } catch (err) {
        console.warn('[SetupBanner] Failed to check KWin setup status:', err)
        // Don't show banner on error - fail silently
      }
    }

    checkSetup()
  }, [])

  const handleFix = useCallback(async () => {
    setBannerState('installing')
    setErrorMsg(null)

    try {
      await invoke('install_kwin_rule')
      setBannerState('success')
    } catch (err) {
      console.error('[SetupBanner] Failed to install KWin rule:', err)
      setErrorMsg(err instanceof Error ? err.message : String(err))
      setBannerState('error')
    }
  }, [])

  const handleDismiss = useCallback(async () => {
    try {
      await invoke('mark_kwin_prompted')
    } catch (err) {
      console.warn('[SetupBanner] Failed to mark as prompted:', err)
    }
    setBannerState('hidden')
  }, [])

  const handleRestart = useCallback(async () => {
    try {
      await relaunch()
    } catch (err) {
      console.error('[SetupBanner] Failed to restart app:', err)
      // Fallback message if restart fails
      setErrorMsg('Please restart the app manually')
    }
  }, [])

  if (bannerState === 'hidden') {
    return null
  }

  if (bannerState === 'success') {
    return (
      <div className={styles.banner} data-state="success">
        <span className={styles.icon}>&#x2714;</span>
        <div className={styles.content}>
          <p className={styles.title}>Setup complete</p>
          <p className={styles.subtitle}>Restart to apply window rules</p>
        </div>
        <div className={styles.actions}>
          <button
            className={styles.restartButton}
            onClick={handleRestart}
            data-no-drag
          >
            Restart Now
          </button>
        </div>
      </div>
    )
  }

  if (bannerState === 'error') {
    return (
      <div className={styles.banner}>
        <span className={styles.icon}>&#x26A0;</span>
        <div className={styles.content}>
          <p className={styles.title}>Setup failed</p>
          <p className={styles.subtitle}>{errorMsg || 'Unknown error'}</p>
        </div>
        <div className={styles.actions}>
          <button
            className={styles.fixButton}
            onClick={handleFix}
            data-no-drag
          >
            Retry
          </button>
          <button
            className={styles.dismissButton}
            onClick={handleDismiss}
            data-no-drag
          >
            Dismiss
          </button>
        </div>
      </div>
    )
  }

  return (
    <div className={styles.banner}>
      <span className={styles.icon}>&#x2699;</span>
      <div className={styles.content}>
        <p className={styles.title}>HUD needs setup for Wayland</p>
        <p className={styles.subtitle}>Enable positioning & always-on-top</p>
      </div>
      <div className={styles.actions}>
        {bannerState === 'installing' ? (
          <button className={styles.fixButton} disabled data-no-drag>
            <span className={styles.spinner} />
          </button>
        ) : (
          <>
            <button
              className={styles.fixButton}
              onClick={handleFix}
              data-no-drag
            >
              Fix Now
            </button>
            <button
              className={styles.dismissButton}
              onClick={handleDismiss}
              data-no-drag
            >
              Dismiss
            </button>
          </>
        )}
      </div>
    </div>
  )
}

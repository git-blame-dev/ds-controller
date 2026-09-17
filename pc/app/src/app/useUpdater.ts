import { useCallback, useEffect, useState } from "react"
import { checkForUpdate, deferUpdate, downloadUpdate, getUpdateSnapshot, installUpdate, setAutoDownloadUpdates, setAutoInstallUpdates } from "./tauriCommands"
import { listenToUpdateSnapshot } from "./tauriEvents"
import type { UpdateOperation, UpdateSnapshot } from "./types"
import { createUpdaterSubscription, INITIAL_UPDATE_SNAPSHOT, retryOperation, selectFreshSnapshot } from "./updaterLogic"

export function useUpdater(onError: (error: unknown) => void) {
  const [snapshot, setSnapshot] = useState<UpdateSnapshot>(INITIAL_UPDATE_SNAPSHOT)
  const applySnapshot = useCallback((incoming: UpdateSnapshot) => {
    setSnapshot((current) => selectFreshSnapshot(current, incoming))
  }, [])
  useEffect(() => {
    const subscription = createUpdaterSubscription({
      listen: listenToUpdateSnapshot,
      getInitialSnapshot: getUpdateSnapshot,
      onSnapshot: applySnapshot,
      onError,
    })
    return subscription.cancel
  }, [applySnapshot, onError])
  const command = useCallback(async (operation: UpdateOperation) => {
    try {
      const next = await ({ check: checkForUpdate, download: downloadUpdate, install: installUpdate }[operation])()
      applySnapshot(next)
    } catch (error) {
      onError(error)
    }
  }, [applySnapshot, onError])
  const defer = useCallback(async () => {
    try {
      applySnapshot(await deferUpdate())
    } catch (error) {
      onError(error)
    }
  }, [applySnapshot, onError])
  const setAutoDownload = useCallback(async (enabled: boolean) => {
    try {
      applySnapshot(await setAutoDownloadUpdates(enabled))
    } catch (error) {
      onError(error)
    }
  }, [applySnapshot, onError])
  const setAutoInstall = useCallback(async (enabled: boolean) => {
    try {
      applySnapshot(await setAutoInstallUpdates(enabled))
    } catch (error) {
      onError(error)
    }
  }, [applySnapshot, onError])

  return {
    snapshot,
    command,
    retry: () => command(retryOperation(snapshot)),
    defer,
    setAutoDownload,
    setAutoInstall,
  }
}

import type { UpdateOperation, UpdateSnapshot } from "./types"

export const INITIAL_UPDATE_SNAPSHOT: UpdateSnapshot = Object.freeze({
  revision: 0,
  phase: "idle",
  operation: null,
  currentVersion: "",
  availableVersion: null,
  notes: null,
  downloadedBytes: 0,
  totalBytes: null,
  error: null,
  autoDownloadEnabled: true,
  autoInstallEnabled: true,
})

interface UpdaterSubscriptionDependencies {
  readonly listen: (handler: (snapshot: UpdateSnapshot) => void) => Promise<() => void>
  readonly getInitialSnapshot: () => Promise<UpdateSnapshot>
  readonly onSnapshot: (snapshot: UpdateSnapshot) => void
  readonly onError: (error: unknown) => void
}

export function selectFreshSnapshot(
  current: UpdateSnapshot | null,
  incoming: UpdateSnapshot,
): UpdateSnapshot {
  return current !== null && current.revision > incoming.revision ? current : incoming
}

export function createUpdaterSubscription(dependencies: UpdaterSubscriptionDependencies) {
  let active = true
  let unlisten: (() => void) | undefined
  const ready = (async () => {
    try {
      const stopListening = await dependencies.listen((snapshot) => {
        if (active) dependencies.onSnapshot(snapshot)
      })
      if (!active) {
        stopListening()
        return
      }
      unlisten = stopListening
      const initial = await dependencies.getInitialSnapshot()
      if (active) dependencies.onSnapshot(initial)
    } catch (error) {
      if (active) dependencies.onError(error)
    }
  })()

  return {
    ready,
    cancel() {
      active = false
      unlisten?.()
    },
  }
}

export function retryOperation(snapshot: UpdateSnapshot): UpdateOperation {
  if (snapshot.phase === "available") return "download"
  if (snapshot.phase === "ready") return "install"
  return "check"
}

export function statusMessage(snapshot: UpdateSnapshot): string {
  if (snapshot.operation === "check") return "Checking for an update…"
  if (snapshot.operation === "download") return "Downloading and verifying the update…"
  if (snapshot.operation === "install") return "Preparing the receiver and starting the installer…"
  if (snapshot.error !== null) return "The update operation failed. Try again."
  if (snapshot.phase === "unavailable") return "Update information is not available yet. Try again later."
  if (snapshot.phase === "available") return `Update ${snapshot.availableVersion ?? ""} is available.`
  if (snapshot.phase === "ready") return "The verified update is ready to install."
  if (snapshot.phase === "restartRequired") return "Restart is required before using the receiver again."
  if (snapshot.phase === "current") return "The desktop app is up to date."
  return "Update status has not been confirmed."
}

export function updateStatusTone(snapshot: UpdateSnapshot): "good" | "warn" | "bad" | "neutral" {
  if (snapshot.error !== null) return "bad"
  if (snapshot.operation !== null) return "warn"
  if (snapshot.phase === "current") return "good"
  return "neutral"
}

export function formatUpdateProgress(snapshot: UpdateSnapshot): string {
  if (snapshot.totalBytes === null) return `${snapshot.downloadedBytes} bytes downloaded`
  return `${snapshot.downloadedBytes} of ${snapshot.totalBytes} bytes downloaded`
}

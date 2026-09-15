import type { UpdateSnapshot } from "../app/types"
import { formatUpdateProgress, statusMessage } from "../app/updaterLogic"
import { StatusBadge } from "./StatusBadge"

type UpdateAction = "check" | "download" | "defer" | "install" | "retry"

interface UpdaterCardProps {
  readonly snapshot: UpdateSnapshot
  readonly onAction: (action: UpdateAction) => void
  readonly onAutoDownload: (enabled: boolean) => void
}

const buttonClass =
  "shrink-0 rounded-lg px-3 py-1.5 text-xs font-semibold focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-cyan-300 disabled:cursor-not-allowed disabled:opacity-50"

export function UpdaterCard({ snapshot, onAction, onAutoDownload }: UpdaterCardProps) {
  const busy = snapshot.operation !== null
  const downloading = snapshot.operation === "download"
  const canRetry = snapshot.error !== null && snapshot.phase !== "restartRequired"
  const progressKnown = snapshot.totalBytes !== null && snapshot.totalBytes > 0

  return (
    <section
      aria-labelledby="updater-title"
      className="relative flex flex-wrap items-center gap-x-3 gap-y-2 rounded-xl border border-white/10 bg-card/80 px-4 py-2 text-xs shadow-xl shadow-black/20"
    >
      <h2 id="updater-title" className="shrink-0 text-sm font-semibold text-white">Updates</h2>
      <span className="text-muted-foreground" aria-label={`Current version ${snapshot.currentVersion || "unknown"}`}>
        v{snapshot.currentVersion || "unknown"}
      </span>
      <div role="status" aria-atomic="true" className="shrink-0">
        <StatusBadge label={statusBadgeLabel(snapshot)} tone={statusTone(snapshot)} />
        <span className="sr-only">{statusMessage(snapshot)}</span>
      </div>
      {downloading && (
        <progress
          aria-label="Update download progress"
          aria-valuetext={formatUpdateProgress(snapshot)}
          title={formatUpdateProgress(snapshot)}
          className="h-1.5 w-20 shrink-0 accent-cyan-300"
          value={progressKnown ? snapshot.downloadedBytes : undefined}
          max={progressKnown ? snapshot.totalBytes ?? undefined : undefined}
        />
      )}
      <div className="ml-auto flex flex-wrap items-center gap-2">
        <label className="mr-1 flex shrink-0 cursor-pointer items-center gap-2 text-slate-200">
          <input
            role="switch"
            type="checkbox"
            aria-label="Automatically check and download updates when the app opens"
            checked={snapshot.autoDownloadEnabled}
            disabled={busy}
            onChange={(event) => onAutoDownload(event.target.checked)}
            className="h-3.5 w-3.5 accent-cyan-300 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-cyan-300"
          />
          Auto-download
        </label>
        {snapshot.phase !== "restartRequired" && snapshot.phase !== "available" && snapshot.phase !== "ready" && (
          <button
            type="button"
            onClick={() => onAction(canRetry ? "retry" : "check")}
            disabled={busy}
            className={`${buttonClass} border border-white/10 bg-white/5 text-white hover:bg-white/10`}
          >
            {canRetry ? "Retry" : "Check now"}
          </button>
        )}
        {snapshot.phase === "available" && (
          <button
            type="button"
            onClick={() => onAction("download")}
            disabled={busy}
            className={`${buttonClass} bg-cyan-300 text-slate-950 hover:bg-cyan-200`}
          >
            {canRetry ? "Retry download" : "Download update"}
          </button>
        )}
        {snapshot.phase === "ready" && (
          <>
            <button
              type="button"
              onClick={() => onAction("install")}
              disabled={busy}
              className={`${buttonClass} bg-cyan-300 text-slate-950 hover:bg-cyan-200`}
            >
              Restart and update
            </button>
            <button
              type="button"
              onClick={() => onAction("defer")}
              disabled={busy}
              className={`${buttonClass} border border-white/10 bg-white/5 text-white hover:bg-white/10`}
            >
              Later
            </button>
          </>
        )}
        <details className="relative">
          <summary className={`${buttonClass} cursor-pointer text-slate-300 hover:bg-white/5 hover:text-white`}>
            Details
          </summary>
          <div className="absolute right-0 top-full z-20 mt-2 max-h-64 w-80 max-w-[calc(100vw-4rem)] space-y-2 overflow-y-auto rounded-xl border border-white/10 bg-card p-4 text-sm text-slate-200 shadow-xl">
            <p>{statusMessage(snapshot)}</p>
            {snapshot.error && <p className="break-words text-red-200">{snapshot.error}</p>}
            {downloading && <p>{formatUpdateProgress(snapshot)}</p>}
            {snapshot.notes && (
              <div>
                <h3 className="font-semibold text-white">Release notes</h3>
                <p className="mt-1 whitespace-pre-wrap break-words">{snapshot.notes}</p>
              </div>
            )}
            <p className="text-xs text-muted-foreground">
              Updates change only the desktop app. The Nintendo DS ROM and its configuration stay unchanged.
            </p>
          </div>
        </details>
      </div>
    </section>
  )
}

function statusBadgeLabel(snapshot: UpdateSnapshot): string {
  if (snapshot.operation !== null) return "Working"
  if (snapshot.error !== null) return "Update error"
  if (snapshot.phase === "unavailable") return "Unavailable"
  if (snapshot.phase === "available") return "Available"
  if (snapshot.phase === "ready") return "Ready"
  if (snapshot.phase === "restartRequired") return "Restart required"
  return "Current"
}

function statusTone(snapshot: UpdateSnapshot): "good" | "warn" | "bad" | "neutral" {
  if (snapshot.error !== null) return "bad"
  if (snapshot.operation !== null || snapshot.phase === "available") return "warn"
  if (snapshot.phase === "ready") return "good"
  return "neutral"
}

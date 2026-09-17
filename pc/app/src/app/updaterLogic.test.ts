import { describe, expect, test } from "vitest"

import type { UpdatePhase, UpdateSnapshot } from "./types"
import { createUpdaterSubscription, formatUpdateProgress, INITIAL_UPDATE_SNAPSHOT, selectFreshSnapshot, retryOperation, statusMessage, updateStatusTone } from "./updaterLogic"

const BASE_SNAPSHOT: UpdateSnapshot = {
  revision: 1,
  phase: "idle",
  operation: null,
  currentVersion: "2026.9.15-10",
  availableVersion: null,
  notes: null,
  downloadedBytes: 0,
  totalBytes: null,
  error: "synthetic network failure",
  autoDownloadEnabled: true,
}

describe("retryOperation", () => {
  test.each([
    ["idle", "check"],
    ["available", "download"],
    ["ready", "install"],
  ] as const)("retries the operation appropriate for %s", (phase, expected) => {
    const snapshot = { ...BASE_SNAPSHOT, phase: phase as UpdatePhase }

    expect(retryOperation(snapshot)).toBe(expected)
  })
})

test("initial updater state mirrors the backend automatic-download default", () => {
  expect(INITIAL_UPDATE_SNAPSHOT.autoDownloadEnabled).toBe(true)
})

test("an idle snapshot with an error does not say the app is up to date", () => {
  expect(statusMessage(BASE_SNAPSHOT)).toContain("failed")
})

test("missing release metadata is reported as temporarily unavailable", () => {
  const snapshot = { ...BASE_SNAPSHOT, phase: "unavailable", error: null } as const

  expect(statusMessage(snapshot)).toContain("not available yet")
})

test("initial update status does not claim the app is current", () => {
  expect(statusMessage({ ...BASE_SNAPSHOT, error: null })).toContain("not been confirmed")
})

test("only a current update status uses the green indicator", () => {
  expect(updateStatusTone({ ...BASE_SNAPSHOT, error: null })).toBe("neutral")
  expect(updateStatusTone({ ...BASE_SNAPSHOT, phase: "current", error: null })).toBe("good")
  expect(updateStatusTone({ ...BASE_SNAPSHOT, phase: "available", error: null })).toBe("neutral")
  expect(updateStatusTone({ ...BASE_SNAPSHOT, phase: "ready", error: null })).toBe("neutral")
  expect(updateStatusTone({ ...BASE_SNAPSHOT, operation: "check", error: null })).toBe("warn")
  expect(updateStatusTone(BASE_SNAPSHOT)).toBe("bad")
})

describe("selectFreshSnapshot", () => {
  test("an initial command response cannot overwrite a newer event", () => {
    const event = { ...BASE_SNAPSHOT, revision: 3, phase: "ready" } as const
    const olderResponse = { ...BASE_SNAPSHOT, revision: 2, phase: "available" } as const

    expect(selectFreshSnapshot(event, olderResponse)).toBe(event)
  })

  test("an overlapping command response cannot overwrite newer progress", () => {
    const progress = { ...BASE_SNAPSHOT, revision: 8, phase: "downloading", downloadedBytes: 512 } as const
    const commandResponse = { ...BASE_SNAPSHOT, revision: 7, phase: "available" } as const

    expect(selectFreshSnapshot(progress, commandResponse)).toBe(progress)
  })
})

describe("createUpdaterSubscription", () => {
  test("an event emitted before the initial response remains authoritative", async () => {
    const event = { ...BASE_SNAPSHOT, revision: 3, phase: "ready" } as const
    const initialResponse = { ...BASE_SNAPSHOT, revision: 2, phase: "available" } as const
    let current: UpdateSnapshot | null = null
    const subscription = createUpdaterSubscription({
      listen: async (handler) => {
        handler(event)
        return () => undefined
      },
      getInitialSnapshot: async () => initialResponse,
      onSnapshot: (incoming) => {
        current = selectFreshSnapshot(current, incoming)
      },
      onError: () => undefined,
    })

    await subscription.ready

    expect(current).toBe(event)
  })

  test("cleanup before listener registration finishes immediately removes the late listener", async () => {
    let resolveListener: ((unlisten: () => void) => void) | undefined
    let cleanupCount = 0
    let snapshotReads = 0
    const subscription = createUpdaterSubscription({
      listen: () => new Promise((resolve) => { resolveListener = resolve }),
      getInitialSnapshot: async () => {
        snapshotReads += 1
        return BASE_SNAPSHOT
      },
      onSnapshot: () => undefined,
      onError: () => undefined,
    })

    subscription.cancel()
    resolveListener?.(() => { cleanupCount += 1 })
    await subscription.ready

    expect(cleanupCount).toBe(1)
    expect(snapshotReads).toBe(0)
  })
})

describe("formatUpdateProgress", () => {
  test("describes downloads whose total size is unknown", () => {
    const snapshot = { ...BASE_SNAPSHOT, phase: "downloading", operation: "download", downloadedBytes: 512 } as const

    expect(formatUpdateProgress(snapshot)).toBe("512 bytes downloaded")
  })
})

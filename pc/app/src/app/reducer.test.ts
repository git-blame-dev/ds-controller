import { describe, expect, test } from "vitest"

import { appReducer, createInitialAppState } from "./reducer"
import type { LogEntry } from "./types"

describe("appReducer", () => {
  test("keeps only the newest 1000 log entries", () => {
    const entries = Array.from({ length: 1001 }, (_, index): LogEntry => ({
      id: `log-${index}`,
      timestamp: "2026-05-13T00:00:00.000Z",
      level: "info",
      message: `entry ${index}`,
    }))

    const state = appReducer(createInitialAppState(), {
      type: "logsReceived",
      entries,
    })

    expect(state.logs).toHaveLength(1000)
    expect(state.logs[0]?.id).toBe("log-1")
    expect(state.logs.at(-1)?.id).toBe("log-1000")
  })

test("tracks unsaved settings when the draft port changes", () => {
const state = appReducer(createInitialAppState(), {
type: "draftSettingsChanged",
settings: { port: 26761 },
})

expect(state.draftSettings.port).toBe(26761)
expect(state.hasUnsavedSettings).toBe(true)
})

test("saves packet logging without discarding unrelated draft settings", () => {
const withDraftPort = appReducer(createInitialAppState(), {
type: "draftSettingsChanged",
settings: { port: 26761 },
})

const state = appReducer(withDraftPort, {
type: "packetLoggingSaved",
enabled: true,
})

expect(state.settings.packetLoggingEnabled).toBe(true)
expect(state.draftSettings.packetLoggingEnabled).toBe(true)
expect(state.settings.port).toBe(26760)
expect(state.draftSettings.port).toBe(26761)
expect(state.hasUnsavedSettings).toBe(true)
})
})

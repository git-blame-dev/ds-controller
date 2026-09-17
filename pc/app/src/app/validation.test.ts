import { describe, expect, test } from "vitest"

import { isUpdateSnapshot, parseRuntimeStatus, validatePortInput } from "./validation"

describe("validatePortInput", () => {
  test("accepts the default receiver port", () => {
    expect(validatePortInput("26760")).toEqual({ ok: true, value: 26760 })
  })

  test("rejects text that is not a whole number", () => {
    expect(validatePortInput("26.760")).toEqual({
      ok: false,
      error: "Port must be a whole number between 1 and 65535.",
    })
  })

  test("rejects ports outside the UDP range", () => {
    expect(validatePortInput("65536")).toEqual({
      ok: false,
      error: "Port must be a whole number between 1 and 65535.",
    })
  })
})

describe("isUpdateSnapshot", () => {
  const snapshot = { revision: 1, phase: "downloading", operation: "download", currentVersion: "1.0.0", availableVersion: "1.1.0", notes: null, downloadedBytes: 10, totalBytes: null, error: null, autoDownloadEnabled: true, autoInstallEnabled: true }
  test("accepts unknown download totals", () => expect(isUpdateSnapshot(snapshot)).toBe(true))
  test("accepts verified current update status", () => expect(isUpdateSnapshot({ ...snapshot, phase: "current", operation: null, availableVersion: null })).toBe(true))
  test("accepts unavailable update metadata", () => expect(isUpdateSnapshot({ ...snapshot, phase: "unavailable", operation: null })).toBe(true))
  test("rejects unknown phases", () => expect(isUpdateSnapshot({ ...snapshot, phase: "broken" })).toBe(false))
  test("rejects snapshots without a revision", () => {
    const withoutRevision = { ...snapshot }
    Reflect.deleteProperty(withoutRevision, "revision")
    expect(isUpdateSnapshot(withoutRevision)).toBe(false)
  })
  test("rejects snapshots without the automatic-install preference", () => {
    const withoutAutoInstall = { ...snapshot }
    Reflect.deleteProperty(withoutAutoInstall, "autoInstallEnabled")
    expect(isUpdateSnapshot(withoutAutoInstall)).toBe(false)
  })
})

describe("parseRuntimeStatus", () => {
test("accepts the canonical frontend runtime status payload", () => {
expect(
parseRuntimeStatus({
receiver: { kind: "idle" },
virtualController: { kind: "unknown" },
pressedButtons: [],
packetCount: 0,
lastPacketAt: null,
}),
).toEqual({
ok: true,
value: {
receiver: { kind: "idle" },
virtualController: { kind: "unknown" },
pressedButtons: [],
packetCount: 0,
lastPacketAt: null,
},
})
})

test("accepts the canonical backend running status payload", () => {
expect(
parseRuntimeStatus({
receiver: { kind: "running", boundAddress: "0.0.0.0:26760", lastSender: null },
virtualController: { kind: "ready" },
pressedButtons: ["a", "start"],
packetCount: 42,
lastPacketAt: "123456",
}),
).toEqual({
ok: true,
value: {
receiver: { kind: "running", boundAddress: "0.0.0.0:26760", lastSender: null },
virtualController: { kind: "ready" },
pressedButtons: ["a", "start"],
packetCount: 42,
lastPacketAt: "123456",
},
})
})

test("rejects non-canonical backend field names", () => {
expect(
parseRuntimeStatus({
receiver: { kind: "Idle" },
virtual_controller: { kind: "Unknown" },
pressed_buttons: [],
packet_count: 0,
last_packet_at: null,
}),
).toEqual({ ok: false, error: "Unknown receiver status kind: Idle" })
})
})

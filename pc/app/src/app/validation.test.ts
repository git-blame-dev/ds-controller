import { describe, expect, test } from "vitest"

import { parseRuntimeStatus, validatePortInput } from "./validation"

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

describe("parseRuntimeStatus", () => {
test("accepts the canonical frontend runtime status payload", () => {
expect(
parseRuntimeStatus({
receiver: { kind: "idle" },
viGem: { kind: "unknown" },
pressedButtons: [],
packetCount: 0,
lastPacketAt: null,
}),
).toEqual({
ok: true,
value: {
receiver: { kind: "idle" },
viGem: { kind: "unknown" },
pressedButtons: [],
packetCount: 0,
lastPacketAt: null,
},
})
})

test("accepts the canonical backend running status payload", () => {
expect(
parseRuntimeStatus({
receiver: { kind: "running", boundAddress: "0.0.0.0:26760", lockedSender: null },
viGem: { kind: "ready" },
pressedButtons: ["a", "start"],
packetCount: 42,
lastPacketAt: "123456",
}),
).toEqual({
ok: true,
value: {
receiver: { kind: "running", boundAddress: "0.0.0.0:26760", lockedSender: null },
viGem: { kind: "ready" },
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
vi_gem: { kind: "Unknown" },
pressed_buttons: [],
packet_count: 0,
last_packet_at: null,
}),
).toEqual({ ok: false, error: "Unknown receiver status kind: Idle" })
})
})

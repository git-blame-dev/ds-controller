import { describe, expect, test } from "vitest"

import { validatePortInput } from "./validation"

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

import { useEffect, useMemo, useReducer, useRef, useState } from "react"

import { appReducer, createInitialAppState } from "./app/reducer"
import { getRuntimeStatus, getSettings, restartReceiver, saveSettings, setPacketLoggingEnabled, startReceiver, stopReceiver } from "./app/tauriCommands"
import { listenToLogEntry, listenToRuntimeStatusChanged, listenToSettingsChanged } from "./app/tauriEvents"
import type { AppSettings, LogEntry } from "./app/types"
import { validatePortInput } from "./app/validation"
import { HeaderBar } from "./components/HeaderBar"
import { LogPanel } from "./components/LogPanel"
import { ReceiverCard } from "./components/ReceiverCard"
import { RuntimeStatusCard } from "./components/RuntimeStatusCard"
import { SenderCard } from "./components/SenderCard"
import { ViGemStatusCard } from "./components/ViGemStatusCard"

export function App() {
const [state, dispatch] = useReducer(appReducer, undefined, createInitialAppState)
const [portInput, setPortInput] = useState(String(state.draftSettings.port))
const packetLoggingSaveChain = useRef<Promise<void>>(Promise.resolve())
const packetLoggingRequestId = useRef(0)
const portValidation = useMemo(() => validatePortInput(portInput), [portInput])

useEffect(() => {
let shouldApplyResponses = true
const unlisteners: Array<() => void> = []

async function boot() {
try {
const settings = await getSettings()
if (!shouldApplyResponses) return
dispatch({ type: "settingsLoaded", settings })
setPortInput(String(settings.port))

const runtimeStatus = await getRuntimeStatus()
if (!shouldApplyResponses) return
dispatch({ type: "runtimeStatusReceived", runtimeStatus })

unlisteners.push(
await listenToSettingsChanged((nextSettings) => {
dispatch({ type: "settingsSaved", settings: nextSettings })
setPortInput(String(nextSettings.port))
}),
await listenToRuntimeStatusChanged((runtimeStatus) => {
dispatch({ type: "runtimeStatusReceived", runtimeStatus })
}),
await listenToLogEntry((entry) => {
dispatch({ type: "logReceived", entry })
}),
)
} catch (error: unknown) {
if (shouldApplyResponses) {
dispatch({ type: "logReceived", entry: createLocalLog("error", describeError(error)) })
}
}
}

void boot()

return () => {
shouldApplyResponses = false
for (const unlisten of unlisteners) {
unlisten()
}
}
}, [])

function handlePortChange(value: string) {
setPortInput(value)
const validation = validatePortInput(value)
if (validation.ok) {
dispatch({ type: "draftSettingsChanged", settings: { port: validation.value } })
}
}

function handleToggle(key: keyof Omit<AppSettings, "port">, value: boolean) {
if (key === "packetLoggingEnabled") {
void handlePacketLoggingToggle(value)
return
}

dispatch({ type: "draftSettingsChanged", settings: { [key]: value } })
}

async function handlePacketLoggingToggle(value: boolean) {
const requestId = packetLoggingRequestId.current + 1
packetLoggingRequestId.current = requestId
const previousValue = state.draftSettings.packetLoggingEnabled
dispatch({ type: "draftSettingsChanged", settings: { packetLoggingEnabled: value } })

packetLoggingSaveChain.current = packetLoggingSaveChain.current.then(async () => {
try {
const savedSettings = await setPacketLoggingEnabled(value)
if (requestId !== packetLoggingRequestId.current) return
dispatch({ type: "packetLoggingSaved", enabled: savedSettings.packetLoggingEnabled })
} catch (error: unknown) {
if (requestId !== packetLoggingRequestId.current) return
await restorePacketLoggingAfterFailure(previousValue)
dispatch({ type: "logReceived", entry: createLocalLog("error", describeError(error)) })
}
})

await packetLoggingSaveChain.current
}

async function restorePacketLoggingAfterFailure(previousValue: boolean) {
try {
const settings = await getSettings()
dispatch({ type: "packetLoggingSaved", enabled: settings.packetLoggingEnabled })
} catch {
dispatch({ type: "draftSettingsChanged", settings: { packetLoggingEnabled: previousValue } })
}
}

async function runCommand(command: () => Promise<unknown>) {
try {
await command()
} catch (error: unknown) {
dispatch({ type: "logReceived", entry: createLocalLog("error", describeError(error)) })
}
}

async function handleApplyRestart() {
if (!portValidation.ok) {
dispatch({ type: "logReceived", entry: createLocalLog("error", portValidation.error) })
return
}

const settings = { ...state.draftSettings, port: portValidation.value }
await runCommand(async () => {
const savedSettings = await saveSettings(settings)
dispatch({ type: "settingsSaved", settings: savedSettings })
const runtimeStatus = await restartReceiver()
dispatch({ type: "runtimeStatusReceived", runtimeStatus })
})
}

return (
<main className="min-h-screen overflow-hidden bg-[radial-gradient(circle_at_top_left,hsl(187_92%_52%_/_0.18),transparent_34%),radial-gradient(circle_at_bottom_right,hsl(262_90%_66%_/_0.16),transparent_32%),hsl(var(--background))] px-6 py-5 text-foreground">
<div className="mx-auto flex h-[calc(100vh-2.5rem)] min-h-[620px] max-w-6xl flex-col gap-5">
<HeaderBar receiver={state.runtimeStatus.receiver} />
<div className="grid min-h-0 flex-1 grid-cols-1 gap-4 lg:grid-cols-[minmax(0,1.45fr)_minmax(280px,0.55fr)]">
<div className="flex min-h-0 flex-col gap-4">
<ReceiverCard
draftSettings={{ ...state.draftSettings, port: portValidation.ok ? portValidation.value : state.draftSettings.port }}
portValue={portInput}
receiver={state.runtimeStatus.receiver}
hasUnsavedSettings={state.hasUnsavedSettings || String(state.draftSettings.port) !== portInput}
portValidation={portValidation}
onPortChange={handlePortChange}
onToggle={handleToggle}
onStart={() => void runCommand(async () => {
const runtimeStatus = await startReceiver()
dispatch({ type: "runtimeStatusReceived", runtimeStatus })
})}
onStop={() => void runCommand(async () => {
const runtimeStatus = await stopReceiver()
dispatch({ type: "runtimeStatusReceived", runtimeStatus })
})}
onApplyRestart={() => void handleApplyRestart()}
/>
<RuntimeStatusCard status={state.runtimeStatus} />
<LogPanel logs={state.logs} />
</div>
<aside className="flex flex-col gap-4">
<ViGemStatusCard status={state.runtimeStatus.viGem} />
<SenderCard receiver={state.runtimeStatus.receiver} lastPacketAt={state.runtimeStatus.lastPacketAt} />
</aside>
</div>
</div>
</main>
)
}

function createLocalLog(level: LogEntry["level"], message: string): LogEntry {
const timestamp = Date.now().toString()
return {
id: `${timestamp}-${Math.random().toString(16).slice(2)}`,
timestamp,
level,
message,
}
}

function describeError(error: unknown): string {
if (error instanceof Error) {
return error.message
}

if (typeof error === "object" && error !== null && "message" in error && typeof error.message === "string") {
return error.message
}

return "Unexpected application error."
}

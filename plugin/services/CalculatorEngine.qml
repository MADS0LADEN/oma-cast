import QtQuick
import Quickshell
import Quickshell.Io

// Long-lived Rust calculator worker (oma-cast-calc serve). Quickshell Process
// I/O is async, so evaluate() queues work and peek() returns the last result
// for a matching query once the reply lands.
Item {
  id: engine

  property string binary: ""
  property bool ready: false

  property string lastQuery: ""
  property string lastRatesKey: ""
  property var lastResult: null
  property bool lastReady: false
  property int revision: 0

  property int nextId: 1
  property int pendingId: 0
  property string pendingQuery: ""
  property string queuedWrite: ""
  property string lastError: ""

  signal message(string level, string text, var metadata)

  function resolveBinary() {
    if (engine.binary) return engine.binary
    // Prefer the installed link; fall back to the in-repo release build / PATH.
    var home = Quickshell.env("HOME") || ""
    if (home) return home + "/.local/bin/oma-cast-calc"
    return "oma-cast-calc"
  }

  // undefined = still computing; null = not a calc query; object = result.
  function peek(query) {
    var q = String(query || "").trim()
    if (!q || q !== engine.lastQuery || !engine.lastReady) return undefined
    return engine.lastResult
  }

  function ensureWorker() {
    if (worker.running) return
    worker.command = [engine.resolveBinary(), "serve"]
    worker.running = true
  }

  function flushWrite() {
    if (!engine.queuedWrite || !worker.running) return
    worker.write(engine.queuedWrite)
    engine.queuedWrite = ""
  }

  function ratesKey(rates) {
    try { return JSON.stringify(rates && typeof rates === "object" ? rates : ({})) }
    catch (e) { return "" }
  }

  function invalidate() {
    engine.lastQuery = ""
    engine.lastRatesKey = ""
    engine.lastResult = null
    engine.lastReady = false
    engine.pendingQuery = ""
    engine.pendingId = 0
    engine.queuedWrite = ""
    engine.revision++
  }

  function evaluate(query, rates) {
    var q = String(query || "").trim()
    var rkey = engine.ratesKey(rates)
    if (!q) return
    if (q === engine.lastQuery && rkey === engine.lastRatesKey && engine.lastReady && q !== engine.pendingQuery)
      return
    if (q === engine.pendingQuery && rkey === engine.lastRatesKey)
      return

    engine.pendingQuery = q
    engine.lastRatesKey = rkey
    engine.pendingId = engine.nextId++
    engine.lastReady = false
    engine.queuedWrite = JSON.stringify({
      id: engine.pendingId,
      query: q,
      rates: rates && typeof rates === "object" ? rates : ({})
    }) + "\n"

    engine.ensureWorker()
    engine.flushWrite()
  }

  function handleLine(line) {
    var text = String(line || "").trim()
    if (!text) return
    try {
      var msg = JSON.parse(text)
      if (!msg || msg.ok === false) {
        engine.lastError = String(msg && msg.error || "calc worker error")
        engine.message("warning", "calculator worker reply failed", { error: engine.lastError })
        return
      }
      if (msg.id !== undefined && msg.id !== null && Number(msg.id) !== engine.pendingId)
        return
      engine.lastQuery = engine.pendingQuery
      engine.lastResult = msg.data === undefined ? null : msg.data
      engine.lastReady = true
      engine.pendingQuery = ""
      engine.pendingId = 0
      engine.revision++
    } catch (e) {
      engine.lastError = String(e)
      engine.message("warning", "calculator worker parse failed", { error: engine.lastError })
    }
  }

  Component.onCompleted: {
    engine.ensureWorker()
    engine.ready = true
  }

  Component.onDestruction: {
    if (worker.running) worker.running = false
  }

  Process {
    id: worker
    running: false
    stdinEnabled: true
    stdout: SplitParser {
      onRead: function(line) { engine.handleLine(line) }
    }
    onStarted: engine.flushWrite()
    onExited: function(exitCode) {
      if (engine.pendingQuery) {
        engine.message("warning", "calculator worker exited", { exitCode: exitCode })
        engine.lastQuery = engine.pendingQuery
        engine.lastResult = null
        engine.lastReady = true
        engine.pendingQuery = ""
        engine.pendingId = 0
        engine.revision++
      }
    }
  }
}

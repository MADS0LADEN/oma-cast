import QtQuick
import Quickshell.Io
import "../core/Schema.js" as Schema

// Daily-cached USD currency rates from frankfurter.app (ECB), persisted in
// currencies.json via JsonStore. On fetch failure, stale on-disk rates are kept.
Item {
  id: ratesService

  required property string path
  property bool ready: false

  property var rates: ({ USD: 1 })
  property double fetchedAt: 0
  property bool fetchInFlight: false
  property bool loading: ratesService.fetchInFlight
  property string lastError: ""
  property bool initialRefreshQueued: false

  readonly property bool loaded: store.loaded

  signal changed()
  signal message(string level, string text, var metadata)

  readonly property int maxAgeMs: 86400000
  readonly property string apiUrl: "https://api.frankfurter.app/latest?from=USD"

  function ratesMap() {
    return ratesService.rates
  }

  function refresh() {
    ratesService.startFetch(true)
  }

  function syncFromStore() {
    if (!store.loaded || !store.doc) return
    var doc = store.doc
    ratesService.rates = doc.rates || ({ USD: 1 })
    ratesService.fetchedAt = Number(doc.fetchedAt || 0)
    ratesService.changed()
    if (!ratesService.initialRefreshQueued) {
      ratesService.initialRefreshQueued = true
      ratesService.startFetch(false)
    }
  }

  function isStale() {
    if (!ratesService.fetchedAt) return true
    return (Date.now() - ratesService.fetchedAt) > ratesService.maxAgeMs
  }

  function startFetch(force) {
    if (ratesService.fetchInFlight) return
    if (!force && !ratesService.isStale()) return
    ratesService.fetchInFlight = true
    ratesService.lastError = ""
    fetchProc.collected = ""
    // Prefer curl: XMLHttpRequest is often present in Quickshell but may be
    // sandboxed; curl is what Omarchy already ships and doctor expects.
    fetchProc.command = ["curl", "-sSL", "-f", "--max-time", "15", "-A", "oma-cast/0.1", ratesService.apiUrl]
    fetchProc.running = true
  }

  function applyFetch(body, ok, errText) {
    if (!ok) {
      ratesService.fetchInFlight = false
      ratesService.lastError = errText || "fetch failed"
      ratesService.message("warning", "currency rates fetch failed; keeping stale rates", { error: ratesService.lastError })
      return
    }
    try {
      var parsed = JSON.parse(body)
      if (!parsed || typeof parsed.rates !== "object")
        throw new Error("unexpected frankfurter response")
      var merged = ({ USD: 1 })
      for (var k in parsed.rates) {
        var n = Number(parsed.rates[k])
        if (Number.isFinite(n)) merged[k] = n
      }
      var now = Date.now()
      var count = 0
      for (var c in merged) count++
      // Update in-memory first so calculator works even if the write is delayed.
      ratesService.rates = merged
      ratesService.fetchedAt = now
      ratesService.changed()
      // Keep fetchInFlight true across store.update so onChanged does not re-sync
      // and re-queue a fetch before fetchedAt is visible.
      var wrote = store.update(function(doc) {
        doc.base = "USD"
        doc.rates = merged
        doc.fetchedAt = now
        doc.source = "frankfurter"
      })
      ratesService.fetchInFlight = false
      if (!wrote)
        ratesService.message("warning", "currency rates fetched but not persisted", { loaded: store.loaded, readOnly: store.readOnly })
      else
        ratesService.message("info", "updated currency rates from frankfurter", { count: count, date: parsed.date || "" })
    } catch (e) {
      ratesService.fetchInFlight = false
      ratesService.lastError = String(e && e.message || e)
      ratesService.message("warning", "currency rates parse failed; keeping stale rates", { error: ratesService.lastError })
    }
  }

  Component.onDestruction: store.flush()

  JsonStore {
    id: store
    path: ratesService.path
    ready: ratesService.ready
    migrations: Schema.documents.currencies.migrations
    onChanged: {
      // Ignore our own post-fetch update; only sync when the file loaded or
      // something else wrote it. fetchInFlight covers the applyFetch→update path.
      if (ratesService.fetchInFlight) return
      // After a successful applyFetch we already set rates/fetchedAt; a store
      // changed from that write would re-enter sync and is harmless, but skip
      // re-queueing refresh via the initialRefreshQueued gate.
      if (ratesService.fetchedAt > 0 && store.doc && Number(store.doc.fetchedAt || 0) === ratesService.fetchedAt)
        return
      ratesService.syncFromStore()
    }
    onMessage: function(level, text, metadata) { ratesService.message(level, text, metadata) }
  }

  // First load: JsonStore sets loaded and emits changed; also hook loaded flip
  // in case changed was skipped (empty file ingest still emits changed).
  Connections {
    target: store
    function onLoadedChanged() {
      if (store.loaded) ratesService.syncFromStore()
    }
  }

  Process {
    id: fetchProc
    property string collected: ""
    stdout: SplitParser { onRead: function(chunk) { fetchProc.collected += chunk } }
    onExited: function(exitCode, exitStatus) {
      var ok = exitStatus === 0 && exitCode === 0
      var body = fetchProc.collected.trim()
      if (!ok || !body)
        ratesService.applyFetch(body, false, "curl exited " + exitCode)
      else
        ratesService.applyFetch(body, true, "")
    }
  }
}

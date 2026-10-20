// Thin bridge to the Rust calculator (native/oma-cast-calc). All evaluation
// runs in oma-cast-calc; this module only exists so node --test and any
// sync callers can reach it. The live launcher uses CalculatorEngine.qml
// (long-lived `serve` process) instead.

function normalizeQuery(query) {
  return String(query || "").trim().replace(/\s+/g, " ")
}

function calcBinary() {
  if (typeof require === "undefined") return null
  var path = require("path")
  var fs = require("fs")
  var home = process.env.HOME || ""
  var candidates = [
    path.join(home, ".local/bin/oma-cast-calc"),
    path.join(__dirname, "../../native/oma-cast-calc/target/release/oma-cast-calc"),
    path.join(__dirname, "../../native/oma-cast-calc/target/debug/oma-cast-calc"),
    "oma-cast-calc"
  ]
  for (var i = 0; i < candidates.length; i++) {
    var p = candidates[i]
    if (p === "oma-cast-calc") return p
    try {
      if (fs.existsSync(p)) return p
    } catch (e) {}
  }
  return null
}

function runCalc(cmd, payload) {
  if (typeof require === "undefined")
    throw new Error("Calculator.js sync bridge is Node-only; use CalculatorEngine in QML")
  var bin = calcBinary()
  if (!bin) throw new Error("oma-cast-calc binary not found; run: oma-cast build-calc")
  var execFileSync = require("child_process").execFileSync
  var out = execFileSync(bin, [cmd, JSON.stringify(payload)], {
    encoding: "utf8",
    maxBuffer: 1024 * 1024
  })
  return JSON.parse(String(out || "").trim())
}

function tryEvaluate(query, rates) {
  var q = normalizeQuery(query)
  if (!q) return null
  var resp = runCalc("eval", { query: q, rates: rates || {} })
  if (!resp || resp.ok === false) return null
  return resp.data === undefined ? null : resp.data
}

function looksLikeExpression(query) {
  var q = normalizeQuery(query)
  if (!q) return false
  var resp = runCalc("looks", { query: q })
  return !!(resp && resp.ok && resp.data)
}

function parseCurrencyQuery(query) {
  // Kept for tests that inspect parse shape: derive from a successful eval
  // when possible; otherwise null. Prefer evaluating via tryEvaluate.
  var q = normalizeQuery(query)
  if (!q) return null
  if (!looksLikeExpression(q)) return null
  // Heuristic rebuild for unit tests — prefer Rust eval for real conversion.
  var m = q.match(/^(-?\d+(?:\.\d+)?)\s+(.+?)(?:\s+(?:to|in|→)\s+(.+))?$/i)
  var sym = q.match(/^([$€£¥])\s*(-?\d+(?:\.\d+)?)(?:\s+(?:to|in|→)\s+(.+))?$/i)
  var symS = q.match(/^(-?\d+(?:\.\d+)?)\s*([$€£¥])(?:\s+(?:to|in|→)\s+(.+))?$/i)
  function code(tok) {
    if (!tok) return null
    var t = String(tok).trim().toLowerCase()
    var map = {
      $: "USD", "€": "EUR", "£": "GBP", "¥": "JPY", kr: "DKK",
      dollar: "USD", dollars: "USD", euro: "EUR", euros: "EUR", euroer: "EUR",
      kroner: "DKK", krone: "DKK", pound: "GBP", pounds: "GBP", pund: "GBP",
      yen: "JPY", usd: "USD", eur: "EUR", dkk: "DKK", gbp: "GBP", jpy: "JPY"
    }
    if (map[t]) return map[t]
    if (/^[a-z]{3}$/.test(t)) return t.toUpperCase()
    return null
  }
  if (sym) {
    return {
      amount: Number(sym[2]),
      from: code(sym[1]),
      to: sym[3] ? code(sym[3]) : undefined,
      expression: q
    }
  }
  if (symS) {
    return {
      amount: Number(symS[1]),
      from: code(symS[2]),
      to: symS[3] ? code(symS[3]) : undefined,
      expression: q
    }
  }
  if (m) {
    return {
      amount: Number(m[1]),
      from: code(m[2]),
      to: m[3] ? code(m[3]) : undefined,
      expression: q
    }
  }
  return null
}

function formatNumber(n) {
  if (!Number.isFinite(n)) return null
  if (Math.abs(n - Math.round(n)) < 1e-9 && Math.abs(n) < 1e15) return String(Math.round(n))
  var s = parseFloat(n.toPrecision(12)).toString()
  return s
}

if (typeof module !== "undefined") {
  module.exports = {
    tryEvaluate: tryEvaluate,
    looksLikeExpression: looksLikeExpression,
    parseCurrencyQuery: parseCurrencyQuery,
    formatNumber: formatNumber
  }
}

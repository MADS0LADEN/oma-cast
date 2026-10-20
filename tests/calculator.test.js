const test = require("node:test")
const assert = require("node:assert/strict")
const C = require("../plugin/core/Calculator.js")

const RATES = { USD: 1, EUR: 0.9, DKK: 6.7, GBP: 0.79, JPY: 150 }

function evalMath(q) {
  const r = C.tryEvaluate(q, RATES)
  assert.ok(r, "expected result for " + q)
  assert.equal(r.kind, "math")
  return r
}

function evalCur(q, rates) {
  const r = C.tryEvaluate(q, rates || RATES)
  assert.ok(r, "expected currency result for " + q)
  assert.equal(r.kind, "currency")
  return r
}

test("looksLikeExpression rejects plain search terms", () => {
  assert.equal(C.looksLikeExpression("firefox"), false)
  assert.equal(C.looksLikeExpression("lock"), false)
  assert.equal(C.tryEvaluate("firefox", RATES), null)
  assert.equal(C.tryEvaluate("lock", RATES), null)
})

test("basic math", () => {
  assert.equal(evalMath("2+2").result, "4")
  assert.equal(evalMath("(1+2)*3").result, "9")
  assert.equal(evalMath("2^10").result, "1024")
  assert.equal(evalMath("10 / 4").result, "2.5")
  assert.equal(evalMath("7 % 3").result, "1")
})

test("functions and constants", () => {
  assert.equal(evalMath("sqrt(16)").result, "4")
  assert.equal(evalMath("abs(-3)").result, "3")
  assert.equal(evalMath("round(2.6)").result, "3")
  assert.equal(evalMath("floor(2.9)").result, "2")
  assert.equal(evalMath("ceil(2.1)").result, "3")
  const pi = evalMath("pi")
  assert.ok(Math.abs(pi.value - Math.PI) < 1e-10)
})

test("rejects invalid math", () => {
  assert.equal(C.tryEvaluate("2+", RATES), null)
  assert.equal(C.tryEvaluate("2 + firefox", RATES), null)
  assert.equal(C.tryEvaluate("eval(1)", RATES), null)
})

test("math result shape", () => {
  const r = evalMath("2+2")
  assert.equal(r.expression, "2+2")
  assert.equal(r.unit, "")
  assert.equal(r.copyText, "4")
  assert.equal(r.value, 4)
})

test("parseCurrencyQuery patterns", () => {
  assert.deepEqual(C.parseCurrencyQuery("100 usd to dkk"), {
    amount: 100,
    from: "USD",
    to: "DKK",
    expression: "100 usd to dkk",
  })
  assert.deepEqual(C.parseCurrencyQuery("$50 to eur"), {
    amount: 50,
    from: "USD",
    to: "EUR",
    expression: "$50 to eur",
  })
  assert.deepEqual(C.parseCurrencyQuery("€50 to usd"), {
    amount: 50,
    from: "EUR",
    to: "USD",
    expression: "€50 to usd",
  })
  assert.deepEqual(C.parseCurrencyQuery("100$ to dkk"), {
    amount: 100,
    from: "USD",
    to: "DKK",
    expression: "100$ to dkk",
  })
  assert.deepEqual(C.parseCurrencyQuery("100 USD in EUR"), {
    amount: 100,
    from: "USD",
    to: "EUR",
    expression: "100 USD in EUR",
  })
  assert.deepEqual(C.parseCurrencyQuery("100 dollars to euros"), {
    amount: 100,
    from: "USD",
    to: "EUR",
    expression: "100 dollars to euros",
  })
  assert.deepEqual(C.parseCurrencyQuery("50 euro"), {
    amount: 50,
    from: "EUR",
    to: undefined,
    expression: "50 euro",
  })
  assert.deepEqual(C.parseCurrencyQuery("100 kroner to pound"), {
    amount: 100,
    from: "DKK",
    to: "GBP",
    expression: "100 kroner to pound",
  })
})

test("currency conversion with frankfurter-style rates", () => {
  // 100 USD → DKK: 100 * 6.7 = 670
  let r = evalCur("100 usd to dkk")
  assert.equal(r.result, "670 DKK")
  assert.equal(r.copyText, "670")
  assert.equal(r.unit, "DKK")

  // $50 → EUR: 50 * 0.9 = 45
  r = evalCur("$50 to eur")
  assert.equal(r.result, "45 EUR")

  // 100 EUR → USD: (100/0.9)*1
  r = evalCur("100 eur to usd")
  assert.ok(Math.abs(r.value - 100 / 0.9) < 0.01)
  assert.match(r.result, /USD$/)

  // 100 usd alone → DKK (default target)
  r = evalCur("100 usd")
  assert.equal(r.unit, "DKK")
  assert.equal(r.result, "670 DKK")

  // 100 dkk alone → USD (source already default)
  r = evalCur("100 dkk")
  assert.equal(r.unit, "USD")
  assert.ok(Math.abs(r.value - 100 / 6.7) < 0.01)

  // Spoken names
  r = evalCur("100 dollars to kroner")
  assert.equal(r.result, "670 DKK")
  r = evalCur("50 euro")
  assert.equal(r.unit, "DKK")
  assert.ok(Math.abs(r.value - (50 / 0.9) * 6.7) < 0.05)
})

test("currency returns null when rate missing", () => {
  assert.equal(C.tryEvaluate("100 xyz to usd", RATES), null)
  assert.equal(C.tryEvaluate("100 usd to xyz", RATES), null)
})

test("looksLikeExpression detects math and currency", () => {
  assert.equal(C.looksLikeExpression("2+2"), true)
  assert.equal(C.looksLikeExpression("100 usd to dkk"), true)
  assert.equal(C.looksLikeExpression("$50"), true)
  assert.equal(C.looksLikeExpression("100 dollars to euros"), true)
})

test("formatNumber trims float noise", () => {
  assert.equal(C.formatNumber(4), "4")
  assert.equal(C.formatNumber(2.5), "2.5")
})

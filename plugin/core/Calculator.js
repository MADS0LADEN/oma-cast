// Pure-JS root-search calculator: safe math and currency conversion (frankfurter-style rates).

var CURRENCY_SYMBOLS = {
  $: "USD",
  "€": "EUR",
  "£": "GBP",
  "¥": "JPY",
}

// Spoken / informal names → ISO codes. Plurals included. "kroner" means DKK
// here because that is the launcher default currency. [D]
var CURRENCY_NAMES = {
  dollar: "USD",
  dollars: "USD",
  buck: "USD",
  bucks: "USD",
  euro: "EUR",
  euros: "EUR",
  euroer: "EUR",
  kroner: "DKK",
  krone: "DKK",
  kronen: "DKK",
  pound: "GBP",
  pounds: "GBP",
  sterling: "GBP",
  pund: "GBP",
  yen: "JPY",
  franc: "CHF",
  francs: "CHF",
  krona: "SEK",
  kronor: "SEK",
  yuan: "CNY",
  renminbi: "CNY",
  rupee: "INR",
  rupees: "INR",
  won: "KRW",
  zloty: "PLN",
  peso: "MXN",
  pesos: "MXN",
  real: "BRL",
  reais: "BRL",
}

var MATH_CONSTANTS = { pi: Math.PI, e: Math.E }

var MATH_FUNCTIONS = {
  sqrt: function (x) {
    return Math.sqrt(x)
  },
  sin: function (x) {
    return Math.sin(x)
  },
  cos: function (x) {
    return Math.cos(x)
  },
  tan: function (x) {
    return Math.tan(x)
  },
  abs: function (x) {
    return Math.abs(x)
  },
  round: function (x) {
    return Math.round(x)
  },
  floor: function (x) {
    return Math.floor(x)
  },
  ceil: function (x) {
    return Math.ceil(x)
  },
  log: function (x) {
    return Math.log10(x)
  },
  ln: function (x) {
    return Math.log(x)
  },
}

function normalizeQuery(query) {
  return String(query || "").trim().replace(/\s+/g, " ")
}

function formatNumber(n) {
  if (!Number.isFinite(n)) return null
  if (Math.abs(n - Math.round(n)) < 1e-9 && Math.abs(n) < 1e15) return String(Math.round(n))
  var s = parseFloat(n.toPrecision(12)).toString()
  if (s.indexOf("e") >= 0 || s.indexOf("E") >= 0) return s
  return s
}

// Currencies: round to cents, drop trailing zeros (90, 666.48).
function formatCurrencyAmount(n) {
  if (!Number.isFinite(n)) return null
  if (Math.abs(n) > 0 && Math.abs(n) < 0.01) return formatNumber(n)
  var fixed = (Math.round(n * 100) / 100).toFixed(2)
  return fixed.replace(/\.?0+$/, "")
}

// Resolve a symbol, ISO code, or spoken name to a 3-letter code (no rates check).
function resolveCurrencyToken(raw) {
  if (raw == null || raw === "") return null
  var s = String(raw).trim()
  if (!s) return null
  if (CURRENCY_SYMBOLS[s]) return CURRENCY_SYMBOLS[s]
  var lower = s.toLowerCase()
  if (lower === "kr") return "DKK"
  if (CURRENCY_NAMES[lower]) return CURRENCY_NAMES[lower]
  if (/^[A-Za-z]{3}$/.test(s)) return s.toUpperCase()
  return null
}

function normalizeCurrencyCode(raw, rates) {
  var code = resolveCurrencyToken(raw)
  if (!code) return null
  if (!rates || rates[code] == null || !Number.isFinite(Number(rates[code]))) return null
  return code
}

function parseAmount(str) {
  if (str == null) return null
  var s = String(str).trim().replace(/,/g, "")
  if (!s || !/^-?\d+(?:\.\d+)?$/.test(s)) return null
  var n = Number(s)
  return Number.isFinite(n) ? n : null
}

// Returns { amount, from, to? } or null.
function parseCurrencyQuery(query) {
  var q = normalizeQuery(query)
  if (!q) return null

  // Currency token: ISO code, symbol name, or spoken name (dollar, euros, …).
  var curTok = "[a-zA-Z$€£¥]{2,16}"

  // $100 to eur / €50 to dollars / £10 in dkk / ¥100 to usd
  var symPrefix = q.match(new RegExp("^([$€£¥])\\s*(-?\\d+(?:\\.\\d+)?)(?:\\s+(?:to|in|→)\\s+(" + curTok + "|kr))?$", "i"))
  if (symPrefix) {
    var fromP = CURRENCY_SYMBOLS[symPrefix[1]]
    var amtP = parseAmount(symPrefix[2])
    if (!fromP || amtP == null) return null
    var toP = symPrefix[3] ? resolveCurrencyToken(symPrefix[3]) : undefined
    if (symPrefix[3] && !toP) return null
    return { amount: amtP, from: fromP, to: toP, expression: q }
  }

  // 100$ to euros
  var symSuffix = q.match(new RegExp("^(-?\\d+(?:\\.\\d+)?)\\s*([$€£¥])(?:\\s+(?:to|in|→)\\s+(" + curTok + "|kr))?$", "i"))
  if (symSuffix) {
    var fromS = CURRENCY_SYMBOLS[symSuffix[2]]
    var amtS = parseAmount(symSuffix[1])
    if (!fromS || amtS == null) return null
    var toS = symSuffix[3] ? resolveCurrencyToken(symSuffix[3]) : undefined
    if (symSuffix[3] && !toS) return null
    return { amount: amtS, from: fromS, to: toS, expression: q }
  }

  // 100 usd to dkk / 100 dollars to euros / 100 kroner
  var words = q.match(
    new RegExp("^(-?\\d+(?:\\.\\d+)?)\\s+(" + curTok + "|kr)\\s*(?:(?:to|in|→)\\s+(" + curTok + "|kr))?$", "i")
  )
  if (words) {
    var amtW = parseAmount(words[1])
    if (amtW == null) return null
    var fromW = resolveCurrencyToken(words[2])
    if (!fromW) return null
    var toW = words[3] ? resolveCurrencyToken(words[3]) : undefined
    if (words[3] && !toW) return null
    return { amount: amtW, from: fromW, to: toW, expression: q }
  }

  return null
}

function looksLikeCurrencyQuery(query) {
  var q = normalizeQuery(query)
  if (!q || !/\d/.test(q)) return false
  if (parseCurrencyQuery(q)) return true
  return false
}

function looksLikeMathQuery(query) {
  var q = normalizeQuery(query)
  if (!q) return false
  if (looksLikeCurrencyQuery(q)) return false
  if (/^[+-]?\d+(?:\.\d+)?$/.test(q)) return true
  var lex = tokenizeMath(q)
  if (lex.error || !lex.tokens || lex.tokens.length === 0) return false
  for (var i = 0; i < lex.tokens.length; i++) {
    var t = lex.tokens[i]
    if (t.type === "number" || t.type === "ident") return true
  }
  return false
}

function looksLikeExpression(query) {
  return looksLikeCurrencyQuery(query) || looksLikeMathQuery(query)
}

function convertCurrency(amount, from, to, rates) {
  var fromCode = normalizeCurrencyCode(from, rates)
  var toCode = normalizeCurrencyCode(to, rates)
  if (!fromCode || !toCode) return null
  var fromRate = Number(rates[fromCode])
  var toRate = Number(rates[toCode])
  if (!(fromRate > 0) || !(toRate > 0)) return null
  var amountInUsd = amount / fromRate
  return amountInUsd * toRate
}

function defaultTargetCurrency(fromCode) {
  // [D] DKK is the default target; if the source is already DKK, convert to USD.
  return fromCode === "DKK" ? "USD" : "DKK"
}

function evaluateCurrency(query, rates) {
  var parsed = parseCurrencyQuery(query)
  if (!parsed) return null

  var fromCode = normalizeCurrencyCode(parsed.from, rates)
  if (!fromCode) return null

  var toCode = parsed.to ? normalizeCurrencyCode(parsed.to, rates) : defaultTargetCurrency(fromCode)
  if (!toCode) return null

  var converted = convertCurrency(parsed.amount, fromCode, toCode, rates)
  if (converted == null || !Number.isFinite(converted)) return null

  var numStr = formatCurrencyAmount(converted)
  if (numStr == null) return null

  var displayExpr = parsed.amount + " " + fromCode + " → " + toCode

  return {
    kind: "currency",
    expression: displayExpr,
    result: numStr + " " + toCode,
    value: Number(numStr),
    unit: toCode,
    copyText: numStr,
  }
}

// --- Safe math lexer/parser ---

function tokenizeMath(input) {
  var s = normalizeQuery(input)
  var i = 0
  var tokens = []

  function peek() {
    return s[i]
  }

  function isDigit(c) {
    return c >= "0" && c <= "9"
  }

  function isIdentStart(c) {
    return (c >= "a" && c <= "z") || (c >= "A" && c <= "Z") || c === "_"
  }

  function isIdentPart(c) {
    return isIdentStart(c) || (c >= "0" && c <= "9")
  }

  while (i < s.length) {
    var c = s[i]
    if (c === " " || c === "\t") {
      i++
      continue
    }
    if (isDigit(c) || (c === "." && isDigit(s[i + 1]))) {
      var start = i
      i++
      while (i < s.length && (isDigit(s[i]) || s[i] === ".")) i++
      tokens.push({ type: "number", value: Number(s.slice(start, i)) })
      continue
    }
    if (isIdentStart(c)) {
      var i0 = i
      i++
      while (i < s.length && isIdentPart(s[i])) i++
      var name = s.slice(i0, i).toLowerCase()
      if (MATH_CONSTANTS[name] !== undefined || MATH_FUNCTIONS[name]) {
        tokens.push({ type: "ident", value: name })
        continue
      }
      return { error: "unknown identifier" }
    }
    if ("+-*/%^(),".indexOf(c) >= 0) {
      tokens.push({ type: c })
      i++
      continue
    }
    return { error: "invalid character" }
  }
  return { tokens: tokens }
}

function parseMathTokens(tokens) {
  var pos = 0

  function cur() {
    return tokens[pos]
  }

  function eat(type) {
    if (cur() && cur().type === type) {
      pos++
      return true
    }
    return false
  }

  function expect(type) {
    if (!eat(type)) return false
    return true
  }

  function parseExpression() {
    return parseAddSub()
  }

  function parseAddSub() {
    var left = parseMulDivMod()
    if (left == null) return null
    while (cur() && (cur().type === "+" || cur().type === "-")) {
      var op = cur().type
      pos++
      var right = parseMulDivMod()
      if (right == null) return null
      left = op === "+" ? left + right : left - right
    }
    return left
  }

  function parseMulDivMod() {
    var left = parsePower()
    if (left == null) return null
    while (cur() && (cur().type === "*" || cur().type === "/" || cur().type === "%")) {
      var op = cur().type
      pos++
      var right = parsePower()
      if (right == null) return null
      if (op === "*") left = left * right
      else if (op === "/") {
        if (right === 0) return null
        left = left / right
      } else {
        if (right === 0) return null
        left = left % right
      }
    }
    return left
  }

  function parsePower() {
    var left = parseUnary()
    if (left == null) return null
    if (cur() && cur().type === "^") {
      pos++
      var right = parsePower()
      if (right == null) return null
      return Math.pow(left, right)
    }
    return left
  }

  function parseUnary() {
    if (eat("+")) return parseUnary()
    if (eat("-")) {
      var v = parseUnary()
      return v == null ? null : -v
    }
    return parsePrimary()
  }

  function parsePrimary() {
    var t = cur()
    if (!t) return null

    if (t.type === "number") {
      pos++
      return t.value
    }

    if (t.type === "ident") {
      var name = t.value
      pos++
      if (eat("(")) {
        if (!MATH_FUNCTIONS[name]) return null
        var arg = parseExpression()
        if (arg == null || !expect(")")) return null
        return MATH_FUNCTIONS[name](arg)
      }
      if (MATH_CONSTANTS[name] !== undefined) return MATH_CONSTANTS[name]
      return null
    }

    if (eat("(")) {
      var inner = parseExpression()
      if (inner == null || !expect(")")) return null
      return inner
    }

    return null
  }

  var result = parseExpression()
  if (result == null || pos !== tokens.length) return null
  if (!Number.isFinite(result)) return null
  return result
}

function evaluateMath(query) {
  if (!looksLikeMathQuery(query)) return null
  var lex = tokenizeMath(query)
  if (lex.error || !lex.tokens || lex.tokens.length === 0) return null
  var value = parseMathTokens(lex.tokens)
  if (value == null) return null
  var numStr = formatNumber(value)
  if (numStr == null) return null
  var expression = normalizeQuery(query)
  return {
    kind: "math",
    expression: expression,
    result: numStr,
    value: Number(numStr),
    unit: "",
    copyText: numStr,
  }
}

function tryEvaluate(query, rates) {
  var q = normalizeQuery(query)
  if (!q) return null

  if (looksLikeCurrencyQuery(q)) {
    var currency = evaluateCurrency(q, rates || {})
    if (currency) return currency
  }

  if (looksLikeMathQuery(q)) {
    return evaluateMath(q)
  }

  return null
}

if (typeof module !== "undefined") {
  module.exports = {
    tryEvaluate: tryEvaluate,
    looksLikeExpression: looksLikeExpression,
    parseCurrencyQuery: parseCurrencyQuery,
    formatNumber: formatNumber,
  }
}

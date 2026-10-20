use serde::Serialize;
use std::collections::HashMap;

const PI: f64 = std::f64::consts::PI;
const E: f64 = std::f64::consts::E;

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CalcResult {
    pub kind: String,
    pub expression: String,
    pub result: String,
    pub value: f64,
    pub unit: String,
    pub copy_text: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ParsedCurrency {
    pub amount: f64,
    pub from: String,
    pub to: Option<String>,
    pub expression: String,
}

fn currency_symbol(code: &str) -> Option<&'static str> {
    match code {
        "$" => Some("USD"),
        "€" => Some("EUR"),
        "£" => Some("GBP"),
        "¥" => Some("JPY"),
        _ => None,
    }
}

fn currency_name(lower: &str) -> Option<&'static str> {
    match lower {
        "dollar" | "dollars" | "buck" | "bucks" => Some("USD"),
        "euro" | "euros" | "euroer" => Some("EUR"),
        "kroner" | "krone" | "kronen" => Some("DKK"),
        "pound" | "pounds" | "sterling" | "pund" => Some("GBP"),
        "yen" => Some("JPY"),
        "franc" | "francs" => Some("CHF"),
        "krona" | "kronor" => Some("SEK"),
        "yuan" | "renminbi" => Some("CNY"),
        "rupee" | "rupees" => Some("INR"),
        "won" => Some("KRW"),
        "zloty" => Some("PLN"),
        "peso" | "pesos" => Some("MXN"),
        "real" | "reais" => Some("BRL"),
        _ => None,
    }
}

pub fn normalize_query(query: &str) -> String {
    query.trim().split_whitespace().collect::<Vec<_>>().join(" ")
}

pub fn format_number(n: f64) -> Option<String> {
    if !n.is_finite() {
        return None;
    }
    if (n - n.round()).abs() < 1e-9 && n.abs() < 1e15 {
        return Some(format!("{}", n.round() as i64));
    }
    let s = format!("{}", format_precision_12(n));
    if s.contains('e') || s.contains('E') {
        return Some(s);
    }
    Some(s)
}

fn format_precision_12(n: f64) -> f64 {
    // Match JS parseFloat(n.toPrecision(12))
    let s = format!("{:.12e}", n);
    s.parse::<f64>().unwrap_or(n)
}

pub fn format_currency_amount(n: f64) -> Option<String> {
    if !n.is_finite() {
        return None;
    }
    if n.abs() > 0.0 && n.abs() < 0.01 {
        return format_number(n);
    }
    let rounded = (n * 100.0).round() / 100.0;
    let fixed = format!("{:.2}", rounded);
    Some(trim_trailing_zeros(&fixed))
}

fn trim_trailing_zeros(s: &str) -> String {
    if !s.contains('.') {
        return s.to_string();
    }
    let mut out = s.to_string();
    while out.ends_with('0') {
        out.pop();
    }
    if out.ends_with('.') {
        out.pop();
    }
    out
}

pub fn resolve_currency_token(raw: &str) -> Option<String> {
    let s = raw.trim();
    if s.is_empty() {
        return None;
    }
    if let Some(code) = currency_symbol(s) {
        return Some(code.to_string());
    }
    let lower = s.to_lowercase();
    if lower == "kr" {
        return Some("DKK".to_string());
    }
    if let Some(code) = currency_name(&lower) {
        return Some(code.to_string());
    }
    if s.len() == 3 && s.chars().all(|c| c.is_ascii_alphabetic()) {
        return Some(s.to_uppercase());
    }
    None
}

fn normalize_currency_code(raw: &str, rates: &HashMap<String, f64>) -> Option<String> {
    let code = resolve_currency_token(raw)?;
    let rate = rates.get(&code)?;
    if rate.is_finite() {
        Some(code)
    } else {
        None
    }
}

pub fn parse_amount(str: &str) -> Option<f64> {
    let s = str.trim().replace(',', "");
    if s.is_empty() {
        return None;
    }
    if !is_amount_str(&s) {
        return None;
    }
    s.parse::<f64>().ok().filter(|n| n.is_finite())
}

fn is_amount_str(s: &str) -> bool {
    let mut chars = s.chars();
    let first = match chars.next() {
        Some('-') => chars.next(),
        Some(c) if c.is_ascii_digit() => Some(c),
        _ => return false,
    };
    if let Some(c) = first {
        if !c.is_ascii_digit() {
            return false;
        }
    } else {
        return false;
    }
    let mut saw_digit = true;
    for c in chars {
        if c.is_ascii_digit() {
            saw_digit = true;
        } else if c == '.' {
            // ok
        } else {
            return false;
        }
    }
    saw_digit
}

fn is_currency_char(c: char) -> bool {
    c.is_ascii_alphabetic() || matches!(c, '$' | '€' | '£' | '¥')
}

fn parse_currency_token(s: &str) -> Option<String> {
    let t = s.trim();
    if t.is_empty() {
        return None;
    }
    if t.eq_ignore_ascii_case("kr") {
        return resolve_currency_token("kr");
    }
    if t.len() < 2 || t.len() > 16 {
        return None;
    }
    if !t.chars().all(is_currency_char) {
        return None;
    }
    resolve_currency_token(t)
}

fn match_sep_and_to(rest: &str) -> Option<Option<String>> {
    let rest = rest.trim();
    if rest.is_empty() {
        return Some(None);
    }
    let lower = rest.to_lowercase();
    for (sep_len, prefix) in [(3, "to "), (3, "in ")] {
        if lower.starts_with(prefix) {
            let raw_to = rest[sep_len..].trim_start();
            let to = parse_currency_token(raw_to)?;
            return Some(Some(to));
        }
    }
    if rest.starts_with('→') {
        let raw_to = rest['→'.len_utf8()..].trim_start();
        let to = parse_currency_token(raw_to)?;
        return Some(Some(to));
    }
    None
}

fn parse_amount_part(s: &str) -> Option<f64> {
    parse_amount(s)
}

pub fn parse_currency_query(query: &str) -> Option<ParsedCurrency> {
    let q = normalize_query(query);
    if q.is_empty() {
        return None;
    }

    // $100 to eur
    for sym in ['$', '€', '£', '¥'] {
        if q.starts_with(sym) {
            let rest = q[sym.len_utf8()..].trim_start();
            if let Some((amt_str, tail)) = split_amount_prefix(rest) {
                let from_p = currency_symbol(&sym.to_string())?;
                let amt = parse_amount_part(amt_str)?;
                let to = match_sep_and_to(tail)?;
                return Some(ParsedCurrency {
                    amount: amt,
                    from: from_p.to_string(),
                    to,
                    expression: q.clone(),
                });
            }
        }
    }

    // 100$ to dkk — suffix symbol
    if let Some(end) = amount_end(&q) {
        let after_amt = q[end..].trim_start();
        if !after_amt.is_empty() {
            let sym = after_amt.chars().next()?;
            if matches!(sym, '$' | '€' | '£' | '¥') {
                let from_s = currency_symbol(&sym.to_string())?;
                let tail = after_amt[sym.len_utf8()..].trim_start();
                let amt = parse_amount_part(&q[..end])?;
                let to = match_sep_and_to(tail)?;
                return Some(ParsedCurrency {
                    amount: amt,
                    from: from_s.to_string(),
                    to,
                    expression: q.clone(),
                });
            }
        }
    }

    // 100 usd to dkk
    if let Some((amt_str, cur_str, tail)) = split_amount_words(&q) {
        let amt = parse_amount_part(amt_str)?;
        let from_w = parse_currency_token(cur_str)?;
        let to = match_sep_and_to(tail)?;
        return Some(ParsedCurrency {
            amount: amt,
            from: from_w,
            to,
            expression: q.clone(),
        });
    }

    None
}

fn split_amount_prefix(s: &str) -> Option<(&str, &str)> {
    let s = s.trim_start();
    let end = amount_end(s)?;
    Some((&s[..end], s[end..].trim_start()))
}

fn amount_end(s: &str) -> Option<usize> {
    if s.is_empty() {
        return None;
    }
    let mut i = 0;
    let bytes = s.as_bytes();
    if bytes[0] == b'-' {
        i = 1;
    }
    if i >= bytes.len() || !bytes[i].is_ascii_digit() {
        return None;
    }
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        i += 1;
    }
    if i < bytes.len() && bytes[i] == b'.' {
        i += 1;
        while i < bytes.len() && bytes[i].is_ascii_digit() {
            i += 1;
        }
    }
    Some(i)
}

fn split_amount_words(q: &str) -> Option<(&str, &str, &str)> {
    let q = q.trim();
    let end = amount_end(q)?;
    let rest = q[end..].trim_start();
    if rest.is_empty() {
        return None;
    }
    let cur_end = currency_token_end(rest)?;
    let cur_str = &rest[..cur_end];
    let tail = rest[cur_end..].trim_start();
    Some((&q[..end], cur_str, tail))
}

fn currency_token_end(s: &str) -> Option<usize> {
    if s.is_empty() {
        return None;
    }
    if s.len() >= 2 && s[..2].eq_ignore_ascii_case("kr") {
        let next = s.get(2..).unwrap_or("");
        if next.is_empty() || next.starts_with(' ') {
            return Some(2);
        }
    }
    let mut i = 0;
    for c in s.chars() {
        if !is_currency_char(c) {
            break;
        }
        i += c.len_utf8();
        if i > 16 {
            return None;
        }
    }
    if i < 2 {
        return None;
    }
    Some(i)
}

pub fn looks_like_currency_query(query: &str) -> bool {
    let q = normalize_query(query);
    if q.is_empty() || !q.chars().any(|c| c.is_ascii_digit()) {
        return false;
    }
    parse_currency_query(&q).is_some()
}

#[derive(Debug, Clone)]
enum Token {
    Number(f64),
    Ident(String),
    Op(char),
}

fn tokenize_math(input: &str) -> Result<Vec<Token>, &'static str> {
    let s = normalize_query(input);
    let bytes = s.as_bytes();
    let mut i = 0;
    let mut tokens = Vec::new();

    while i < bytes.len() {
        let c = bytes[i] as char;
        if c == ' ' || c == '\t' {
            i += 1;
            continue;
        }
        if c.is_ascii_digit() || (c == '.' && i + 1 < bytes.len() && bytes[i + 1].is_ascii_digit()) {
            let start = i;
            i += 1;
            while i < bytes.len() && (bytes[i].is_ascii_digit() || bytes[i] == b'.') {
                i += 1;
            }
            let num: f64 = s[start..i].parse().unwrap_or(f64::NAN);
            tokens.push(Token::Number(num));
            continue;
        }
        if is_ident_start(c) {
            let start = i;
            i += 1;
            while i < bytes.len() && is_ident_part(bytes[i] as char) {
                i += 1;
            }
            let name = s[start..i].to_lowercase();
            if is_math_constant(&name) || is_math_function(&name) {
                tokens.push(Token::Ident(name));
                continue;
            }
            return Err("unknown identifier");
        }
        if "+-*/%^(),".contains(c) {
            tokens.push(Token::Op(c));
            i += 1;
            continue;
        }
        return Err("invalid character");
    }
    Ok(tokens)
}

fn is_ident_start(c: char) -> bool {
    c.is_ascii_alphabetic() || c == '_'
}

fn is_ident_part(c: char) -> bool {
    is_ident_start(c) || c.is_ascii_digit()
}

fn is_math_constant(name: &str) -> bool {
    matches!(name, "pi" | "e")
}

fn is_math_function(name: &str) -> bool {
    matches!(
        name,
        "sqrt" | "sin" | "cos" | "tan" | "abs" | "round" | "floor" | "ceil" | "log" | "ln"
    )
}

fn math_constant(name: &str) -> Option<f64> {
    match name {
        "pi" => Some(PI),
        "e" => Some(E),
        _ => None,
    }
}

fn math_function(name: &str, x: f64) -> Option<f64> {
    let v = match name {
        "sqrt" => x.sqrt(),
        "sin" => x.sin(),
        "cos" => x.cos(),
        "tan" => x.tan(),
        "abs" => x.abs(),
        "round" => x.round(),
        "floor" => x.floor(),
        "ceil" => x.ceil(),
        "log" => x.log10(),
        "ln" => x.ln(),
        _ => return None,
    };
    if v.is_finite() {
        Some(v)
    } else {
        None
    }
}

struct Parser<'a> {
    tokens: &'a [Token],
    pos: usize,
}

impl<'a> Parser<'a> {
    fn cur(&self) -> Option<&Token> {
        self.tokens.get(self.pos)
    }

    fn eat(&mut self, op: char) -> bool {
        if let Some(Token::Op(c)) = self.cur() {
            if *c == op {
                self.pos += 1;
                return true;
            }
        }
        false
    }

    fn expect(&mut self, op: char) -> bool {
        self.eat(op)
    }

    fn parse_expression(&mut self) -> Option<f64> {
        self.parse_add_sub()
    }

    fn parse_add_sub(&mut self) -> Option<f64> {
        let mut left = self.parse_mul_div_mod()?;
        loop {
            let op = match self.cur() {
                Some(Token::Op(c)) if *c == '+' || *c == '-' => *c,
                _ => break,
            };
            self.pos += 1;
            let right = self.parse_mul_div_mod()?;
            left = if op == '+' { left + right } else { left - right };
        }
        Some(left)
    }

    fn parse_mul_div_mod(&mut self) -> Option<f64> {
        let mut left = self.parse_power()?;
        loop {
            let op = match self.cur() {
                Some(Token::Op(c)) if matches!(*c, '*' | '/' | '%') => *c,
                _ => break,
            };
            self.pos += 1;
            let right = self.parse_power()?;
            left = match op {
                '*' => left * right,
                '/' => {
                    if right == 0.0 {
                        return None;
                    }
                    left / right
                }
                '%' => {
                    if right == 0.0 {
                        return None;
                    }
                    left % right
                }
                _ => unreachable!(),
            };
        }
        Some(left)
    }

    fn parse_power(&mut self) -> Option<f64> {
        let left = self.parse_unary()?;
        if self.eat('^') {
            let right = self.parse_power()?;
            return Some(left.powf(right));
        }
        Some(left)
    }

    fn parse_unary(&mut self) -> Option<f64> {
        if self.eat('+') {
            return self.parse_unary();
        }
        if self.eat('-') {
            let v = self.parse_unary()?;
            return Some(-v);
        }
        self.parse_primary()
    }

    fn parse_primary(&mut self) -> Option<f64> {
        let t = self.cur()?.clone();
        match t {
            Token::Number(n) => {
                self.pos += 1;
                Some(n)
            }
            Token::Ident(name) => {
                self.pos += 1;
                if self.eat('(') {
                    if !is_math_function(&name) {
                        return None;
                    }
                    let arg = self.parse_expression()?;
                    if !self.expect(')') {
                        return None;
                    }
                    return math_function(&name, arg);
                }
                math_constant(&name)
            }
            Token::Op('(') => {
                self.pos += 1;
                let inner = self.parse_expression()?;
                if !self.expect(')') {
                    return None;
                }
                Some(inner)
            }
            _ => None,
        }
    }
}

fn parse_math_tokens(tokens: &[Token]) -> Option<f64> {
    let mut p = Parser {
        tokens,
        pos: 0,
    };
    let result = p.parse_expression()?;
    if p.pos != tokens.len() {
        return None;
    }
    if result.is_finite() {
        Some(result)
    } else {
        None
    }
}

pub fn looks_like_math_query(query: &str) -> bool {
    let q = normalize_query(query);
    if q.is_empty() {
        return false;
    }
    if looks_like_currency_query(&q) {
        return false;
    }
    if is_plain_number(&q) {
        return true;
    }
    match tokenize_math(&q) {
        Ok(tokens) if !tokens.is_empty() => tokens
            .iter()
            .any(|t| matches!(t, Token::Number(_) | Token::Ident(_))),
        _ => false,
    }
}

fn is_plain_number(q: &str) -> bool {
    let s = q.trim();
    if s.is_empty() {
        return false;
    }
    let mut chars = s.chars();
    match chars.next() {
        Some('+') | Some('-') => {}
        Some(c) if c.is_ascii_digit() => {}
        _ => return false,
    }
    is_amount_str(s.trim_start_matches(['+', '-']))
}

pub fn looks_like_expression(query: &str) -> bool {
    looks_like_currency_query(query) || looks_like_math_query(query)
}

fn convert_currency(amount: f64, from: &str, to: &str, rates: &HashMap<String, f64>) -> Option<f64> {
    let from_code = normalize_currency_code(from, rates)?;
    let to_code = normalize_currency_code(to, rates)?;
    let from_rate = *rates.get(&from_code)?;
    let to_rate = *rates.get(&to_code)?;
    if !(from_rate > 0.0) || !(to_rate > 0.0) {
        return None;
    }
    let amount_in_usd = amount / from_rate;
    Some(amount_in_usd * to_rate)
}

fn default_target_currency(from_code: &str) -> &'static str {
    if from_code == "DKK" {
        "USD"
    } else {
        "DKK"
    }
}

fn evaluate_currency(query: &str, rates: &HashMap<String, f64>) -> Option<CalcResult> {
    let parsed = parse_currency_query(query)?;
    let from_code = normalize_currency_code(&parsed.from, rates)?;
    let to_code = if parsed.to.is_some() {
        normalize_currency_code(parsed.to.as_deref().unwrap(), rates)?
    } else {
        normalize_currency_code(default_target_currency(&from_code), rates)?
    };

    let converted = convert_currency(parsed.amount, &from_code, &to_code, rates)?;
    if !converted.is_finite() {
        return None;
    }
    let num_str = format_currency_amount(converted)?;
    let display_expr = format!("{} {} → {}", parsed.amount, from_code, to_code);
    let value: f64 = num_str.parse().ok()?;

    Some(CalcResult {
        kind: "currency".to_string(),
        expression: display_expr,
        result: format!("{} {}", num_str, to_code),
        value,
        unit: to_code,
        copy_text: num_str,
    })
}

fn evaluate_math(query: &str) -> Option<CalcResult> {
    if !looks_like_math_query(query) {
        return None;
    }
    let tokens = tokenize_math(query).ok()?;
    if tokens.is_empty() {
        return None;
    }
    let value = parse_math_tokens(&tokens)?;
    let num_str = format_number(value)?;
    let expression = normalize_query(query);
    let value_parsed: f64 = num_str.parse().ok()?;

    Some(CalcResult {
        kind: "math".to_string(),
        expression,
        result: num_str.clone(),
        value: value_parsed,
        unit: String::new(),
        copy_text: num_str,
    })
}

pub fn try_evaluate(query: &str, rates: &HashMap<String, f64>) -> Option<CalcResult> {
    let q = normalize_query(query);
    if q.is_empty() {
        return None;
    }

    if looks_like_currency_query(&q) {
        if let Some(currency) = evaluate_currency(&q, rates) {
            return Some(currency);
        }
    }

    if looks_like_math_query(&q) {
        return evaluate_math(&q);
    }

    None
}

pub fn rates_from_json(value: &serde_json::Value) -> HashMap<String, f64> {
    let mut map = HashMap::new();
    if let Some(obj) = value.as_object() {
        for (k, v) in obj {
            if let Some(n) = v.as_f64() {
                map.insert(k.to_uppercase(), n);
            } else if let Some(n) = v.as_i64() {
                map.insert(k.to_uppercase(), n as f64);
            } else if let Some(n) = v.as_u64() {
                map.insert(k.to_uppercase(), n as f64);
            }
        }
    }
    map
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rates() -> HashMap<String, f64> {
        HashMap::from([
            ("USD".into(), 1.0),
            ("EUR".into(), 0.9),
            ("DKK".into(), 6.7),
            ("GBP".into(), 0.79),
            ("JPY".into(), 150.0),
        ])
    }

    fn eval_math(q: &str) -> CalcResult {
        let r = try_evaluate(q, &rates()).expect("expected result");
        assert_eq!(r.kind, "math");
        r
    }

    fn eval_cur(q: &str, custom_rates: Option<&HashMap<String, f64>>) -> CalcResult {
        let default = rates();
        let r = try_evaluate(q, custom_rates.unwrap_or(&default)).expect("expected currency result");
        assert_eq!(r.kind, "currency");
        r
    }

    #[test]
    fn rejects_plain_search_terms() {
        assert!(!looks_like_expression("firefox"));
        assert!(!looks_like_expression("lock"));
        assert!(try_evaluate("firefox", &rates()).is_none());
        assert!(try_evaluate("lock", &rates()).is_none());
    }

    #[test]
    fn basic_math() {
        assert_eq!(eval_math("2+2").result, "4");
        assert_eq!(eval_math("(1+2)*3").result, "9");
        assert_eq!(eval_math("2^10").result, "1024");
        assert_eq!(eval_math("10 / 4").result, "2.5");
        assert_eq!(eval_math("7 % 3").result, "1");
    }

    #[test]
    fn functions_and_constants() {
        assert_eq!(eval_math("sqrt(16)").result, "4");
        assert_eq!(eval_math("abs(-3)").result, "3");
        assert_eq!(eval_math("round(2.6)").result, "3");
        assert_eq!(eval_math("floor(2.9)").result, "2");
        assert_eq!(eval_math("ceil(2.1)").result, "3");
        let pi = eval_math("pi");
        assert!((pi.value - PI).abs() < 1e-10);
    }

    #[test]
    fn rejects_invalid_math() {
        assert!(try_evaluate("2+", &rates()).is_none());
        assert!(try_evaluate("2 + firefox", &rates()).is_none());
        assert!(try_evaluate("eval(1)", &rates()).is_none());
    }

    #[test]
    fn math_result_shape() {
        let r = eval_math("2+2");
        assert_eq!(r.expression, "2+2");
        assert_eq!(r.unit, "");
        assert_eq!(r.copy_text, "4");
        assert_eq!(r.value, 4.0);
    }

    #[test]
    fn parse_currency_query_patterns() {
        assert_eq!(
            parse_currency_query("100 usd to dkk"),
            Some(ParsedCurrency {
                amount: 100.0,
                from: "USD".into(),
                to: Some("DKK".into()),
                expression: "100 usd to dkk".into(),
            })
        );
        assert_eq!(
            parse_currency_query("$50 to eur"),
            Some(ParsedCurrency {
                amount: 50.0,
                from: "USD".into(),
                to: Some("EUR".into()),
                expression: "$50 to eur".into(),
            })
        );
        assert_eq!(
            parse_currency_query("€50 to usd"),
            Some(ParsedCurrency {
                amount: 50.0,
                from: "EUR".into(),
                to: Some("USD".into()),
                expression: "€50 to usd".into(),
            })
        );
        assert_eq!(
            parse_currency_query("100$ to dkk"),
            Some(ParsedCurrency {
                amount: 100.0,
                from: "USD".into(),
                to: Some("DKK".into()),
                expression: "100$ to dkk".into(),
            })
        );
        assert_eq!(
            parse_currency_query("100 USD in EUR"),
            Some(ParsedCurrency {
                amount: 100.0,
                from: "USD".into(),
                to: Some("EUR".into()),
                expression: "100 USD in EUR".into(),
            })
        );
        assert_eq!(
            parse_currency_query("100 dollars to euros"),
            Some(ParsedCurrency {
                amount: 100.0,
                from: "USD".into(),
                to: Some("EUR".into()),
                expression: "100 dollars to euros".into(),
            })
        );
        assert_eq!(
            parse_currency_query("50 euro"),
            Some(ParsedCurrency {
                amount: 50.0,
                from: "EUR".into(),
                to: None,
                expression: "50 euro".into(),
            })
        );
        assert_eq!(
            parse_currency_query("100 kroner to pound"),
            Some(ParsedCurrency {
                amount: 100.0,
                from: "DKK".into(),
                to: Some("GBP".into()),
                expression: "100 kroner to pound".into(),
            })
        );
    }

    #[test]
    fn currency_conversion() {
        let r = eval_cur("100 usd to dkk", None);
        assert_eq!(r.result, "670 DKK");
        assert_eq!(r.copy_text, "670");
        assert_eq!(r.unit, "DKK");

        let r = eval_cur("$50 to eur", None);
        assert_eq!(r.result, "45 EUR");

        let r = eval_cur("100 eur to usd", None);
        assert!((r.value - 100.0 / 0.9).abs() < 0.01);
        assert!(r.result.ends_with("USD"));

        let r = eval_cur("100 usd", None);
        assert_eq!(r.unit, "DKK");
        assert_eq!(r.result, "670 DKK");

        let r = eval_cur("100 dkk", None);
        assert_eq!(r.unit, "USD");
        assert!((r.value - 100.0 / 6.7).abs() < 0.01);

        let r = eval_cur("100 dollars to kroner", None);
        assert_eq!(r.result, "670 DKK");

        let r = eval_cur("50 euro", None);
        assert_eq!(r.unit, "DKK");
        assert!((r.value - (50.0 / 0.9) * 6.7).abs() < 0.05);
    }

    #[test]
    fn currency_null_when_rate_missing() {
        assert!(try_evaluate("100 xyz to usd", &rates()).is_none());
        assert!(try_evaluate("100 usd to xyz", &rates()).is_none());
    }

    #[test]
    fn looks_like_expression_detects() {
        assert!(looks_like_expression("2+2"));
        assert!(looks_like_expression("100 usd to dkk"));
        assert!(looks_like_expression("$50"));
        assert!(looks_like_expression("100 dollars to euros"));
    }

    #[test]
    fn format_number_trims() {
        assert_eq!(format_number(4.0), Some("4".into()));
        assert_eq!(format_number(2.5), Some("2.5".into()));
    }
}

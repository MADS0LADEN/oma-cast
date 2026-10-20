use oma_cast_calc::{rates_from_json, try_evaluate};
use serde::Serialize;
use serde_json::{json, Value};
use std::io::{self, BufRead, Read, Write};

#[derive(Serialize)]
struct EvalResponse {
    ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    id: Option<u64>,
    data: Option<oma_cast_calc::CalcResult>,
}

fn run_eval(input: &str) -> EvalResponse {
    let v: Value = match serde_json::from_str(input) {
        Ok(v) => v,
        Err(_) => return EvalResponse {
            ok: false,
            id: None,
            data: None,
        },
    };
    let query = v.get("query").and_then(|q| q.as_str()).unwrap_or("");
    let rates = v
        .get("rates")
        .map(rates_from_json)
        .unwrap_or_default();
    EvalResponse {
        ok: true,
        id: None,
        data: try_evaluate(query, &rates),
    }
}

fn run_serve_line(line: &str) -> String {
    let v: Value = match serde_json::from_str(line) {
        Ok(v) => v,
        Err(e) => {
            return json!({"ok": false, "error": e.to_string()}).to_string();
        }
    };

    let id = v.get("id").and_then(|i| i.as_u64());
    let query = v
        .get("query")
        .or_else(|| v.get("evaluate"))
        .and_then(|q| q.as_str())
        .unwrap_or("");
    let rates = v
        .get("rates")
        .map(rates_from_json)
        .unwrap_or_default();

    let resp = EvalResponse {
        ok: true,
        id,
        data: try_evaluate(query, &rates),
    };
    serde_json::to_string(&resp).unwrap_or_else(|_| "{\"ok\":false}".into())
}

fn cmd_eval(args: &[String]) {
    let input = if args.is_empty() {
        let mut buf = String::new();
        io::stdin()
            .read_to_string(&mut buf)
            .expect("read stdin");
        buf
    } else {
        args.join(" ")
    };

    let resp = run_eval(input.trim());
    println!("{}", serde_json::to_string(&resp).unwrap());
}

fn cmd_serve() {
    let stdin = io::stdin();
    let mut stdout = io::stdout();
    for line in stdin.lock().lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => break,
        };
        if line.trim().is_empty() {
            continue;
        }
        let out = run_serve_line(line.trim());
        writeln!(stdout, "{out}").ok();
        stdout.flush().ok();
    }
}

fn cmd_looks(args: &[String]) {
    use oma_cast_calc::looks_like_expression;
    let input = if args.is_empty() {
        let mut buf = String::new();
        io::stdin().read_to_string(&mut buf).expect("read stdin");
        buf
    } else {
        args.join(" ")
    };
    let v: Value = serde_json::from_str(input.trim()).unwrap_or(json!({}));
    let query = v.get("query").and_then(|q| q.as_str()).unwrap_or("");
    println!(
        "{}",
        json!({"ok": true, "data": looks_like_expression(query)})
    );
}

fn main() {
    let mut args = std::env::args().skip(1).collect::<Vec<_>>();
    if args.is_empty() {
        eprintln!("usage: oma-cast-calc eval [json] | looks [json] | serve");
        std::process::exit(1);
    }
    let cmd = args.remove(0);
    match cmd.as_str() {
        "eval" => cmd_eval(&args),
        "looks" => cmd_looks(&args),
        "serve" => cmd_serve(),
        _ => {
            eprintln!("unknown command: {cmd}");
            std::process::exit(1);
        }
    }
}

# oma-cast-calc

Native Rust port of oma-cast’s safe math and currency calculator (Frankfurter-style rates).

## Build

```bash
cd native/oma-cast-calc && cargo build --release
```

## Run

```bash
# One-shot eval (argument or stdin JSON: {"query":"...","rates":{...}})
./target/release/oma-cast-calc eval '{"query":"2+2","rates":{"USD":1,"DKK":6.7}}'
echo '{"query":"100 dollars to kroner","rates":{"USD":1,"DKK":6.7,"EUR":0.9}}' | ./target/release/oma-cast-calc eval

# NDJSON server (one JSON object per line on stdin)
./target/release/oma-cast-calc serve
```

## Test

```bash
cargo test
```

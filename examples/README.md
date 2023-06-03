### Create Self-Signed certificate

```bash
openssl req -x509 -newkey rsa:2048 -nodes -sha256 -subj "/CN=localhost" -keyout key.pem -out cert.pem
```

- Run example: `cargo run --example hello_world`

Goto https://127.0.0.1:4433/ or run `curl -k https://127.0.0.1:4433`
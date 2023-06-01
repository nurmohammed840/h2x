### Create Self-Signed certificate

```bash
openssl req -x509 -newkey rsa:4096 -nodes -sha256 -days 365 -subj "/CN=localhost" -keyout key.pem -out cert.pem
```

- Run example: `cargo run --example hello_world`
- Goto: https://127.0.0.1:4433/
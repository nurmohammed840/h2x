### Create Self-Signed certificate

```bash
openssl req -x509 -newkey rsa:2048 -nodes -sha256 -subj "/CN=localhost" -keyout key.pem -out cert.pem
```

### Run example

Run server with:

```
cargo run --example graceful_shutdown
```

Goto https://localhost:4433/ or run `curl -k https://127.0.0.1:4433`


### Demo

Run Server:

```
cargo run --example hello_world
```

Run Client:

```
curl -k -X POST https://127.0.0.1:4433/ -d 'Hello World!'
```

Client output:

```
RequestParts {
    method: POST,
    uri: https://127.0.0.1:4433/,
    version: HTTP/2.0,
    headers: {
        "user-agent": "curl/8.0.1",
        "accept": "*/*",
        "content-length": "12",
        "content-type": "application/x-www-form-urlencoded",
    },
}
Hello World!
```
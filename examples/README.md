### Create self-signed certificate

```bash
openssl req -x509 -newkey rsa:4096 -nodes -sha256 -days 365 -subj "/CN=localhost" -keyout key.pem -out cert.pem
```

- [Allows invalid certificate for localhost](chrome://flags/#allow-insecure-localhost)


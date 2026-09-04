# Security

## Secrets

Never commit API keys, access tokens, credentials, private keys, local databases, or `.env` files. Tom stores AI provider keys in the operating system credential store; SQLite contains only non-secret provider configuration such as provider name, endpoint, and local model path.

If a secret is committed accidentally, revoke it immediately, remove it from Git history, and rotate every affected credential before publishing again.

## Local data

Runtime databases live in the operating system's local application-data directory and are ignored by Git. Repository tests use temporary or in-memory databases.

## Reporting

Until a private security contact is configured, do not open public issues containing secrets, personal data, or exploit details.


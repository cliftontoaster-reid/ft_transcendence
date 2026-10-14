# Authentication

Authentication uses OpenID Connect (OIDC) with the authorization-code flow,
PKCE, and Redis-backed login transactions.

## Login flow

1. The client requests `/auth/login/<provider>`.
2. The service validates the provider and configured redirect URI.
3. Provider metadata is discovered from the issuer.
4. The service creates state, nonce, and PKCE values.
5. The login transaction is stored in Redis for ten minutes.
6. The client is redirected to the provider.
7. The provider redirects to `/auth/callback` with `code` and `state`.
8. The service atomically consumes the Redis transaction.
9. The authorization code is exchanged and the ID token is verified.
10. The service returns the authenticated provider subject and profile claims.

The application layer is responsible for mapping that authenticated identity
to a local `users` row and issuing an application session.

## Security properties

- Redirect URIs must be explicitly configured.
- A provider with multiple redirect URIs requires the caller to select one.
- The `openid` scope is mandatory.
- PKCE protects the authorization-code exchange.
- State prevents login CSRF and is consumed once.
- The nonce is checked against the verified ID token.
- Redis keys contain a hash of the state rather than the raw state.

Do not put client secrets in source control. Prefer the service's configured
secret-reference mechanism and keep local configuration outside the repository.

## Troubleshooting

| Symptom | Likely cause |
| --- | --- |
| Unknown provider | Provider name is absent from OIDC configuration |
| Invalid redirect URI | URI does not exactly match configured values |
| Missing login transaction | Redis is unavailable or the transaction expired |
| State mismatch | Callback state was altered or belongs to another login |
| ID-token verification failure | Issuer, nonce, client, or provider metadata mismatch |

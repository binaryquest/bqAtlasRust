# Authentication and accounts

Choose exactly one browser identity mode with `BQATLAS_AUTH_MODE=local|oidc`. Startup rejects unknown modes and invalid OIDC configuration; it never falls back from external login to passwords.

## Local development

`bootstrap` is an explicit Development-only command, using `BQATLAS_BOOTSTRAP_EMAIL` and `BQATLAS_BOOTSTRAP_PASSWORD`. It creates a confirmed account with the demo's declared permissions and never changes an existing account. Passwords use salted Argon2id PHC hashes with bounded hashing concurrency. Login is rate-limited by the direct peer address, with account lockout after repeated failures. Forwarding headers are not trusted for rate limits.

Sessions are stored in the configured database. The Rust cookie has a distinct name, is HttpOnly and SameSite=Lax, and is Secure outside Development. There is an eight-hour absolute local ticket lifetime. Password change/reset changes the user's security version; every request rechecks it, invalidating other old sessions. Logout deletes the session, and an old concurrent save cannot recreate it.

`GET /api/v1/session/csrf` creates a session-bound token. Every browser POST/PUT/DELETE, including queries and anonymous login/recovery, requires `X-BQATLAS-CSRF`. Successful login rotates session and CSRF. OIDC logout also accepts Atlas's URL-encoded `__RequestVerificationToken` form field at `/auth/logout` so the browser can follow the provider redirect.

Public registration is disabled. `create-user --permissions permissions.json --name "Operator"` deliberately provisions a new local account using `BQATLAS_ACCOUNT_EMAIL` and `BQATLAS_ACCOUNT_PASSWORD` (keep secrets out of shell history). The JSON file is an explicit array of declared permission IDs. It preserves existing accounts. Add `--confirmed` only when deliberately marking the address verified; otherwise configure a sender and request confirmation. This command is available in Production and has no implicit administrator role. The Development bootstrap remains unavailable there.

## Password reset and confirmation

Set `BQATLAS_DEV_MAIL_DIR=.local-mail` and `BQATLAS_PUBLIC_ORIGIN=http://127.0.0.1:4300` to opt into the development mail sink. Its directory/files are private on Unix and ignored by Git. Reset/confirmation links are compatible with the shared Atlas account-recovery screen.

`POST /auth/request-password-reset` and `/auth/request-confirmation` accept `{ "email": "..." }` and return the same 202 acknowledgement for known/unknown/ineligible accounts. Tokens are random, stored hashed, bound to purpose/user/security version, expire after an hour and are consumed atomically. Password reset takes `{userId, token, newPassword}`; confirmation takes `{userId, token}`. Used, expired and wrong-purpose tokens fail. Newer requests invalidate the previous link for that purpose.

Production applications inject `RecoveryConfig` with an implementation of `AccountEmailSender`; configure an HTTPS public origin. Development file delivery refuses Production. Unconfigured delivery returns 503; it does not pretend email was sent. Delivery failures are logged without links or tokens and retain the public generic acknowledgement.

## OpenID Connect

Start the separate Development provider:

```sh
docker compose -f dev/compose.yaml --profile oidc up -d keycloak
```

Configure `.env`:

```dotenv
BQATLAS_AUTH_MODE=oidc
BQATLAS_PUBLIC_ORIGIN=http://127.0.0.1:4300
BQATLAS_OIDC_ISSUER=http://127.0.0.1:28182/realms/bqatlas-rust
BQATLAS_OIDC_CLIENT_ID=bqatlas-rust
BQATLAS_OIDC_CLIENT_SECRET=rust-oidc-dev-only
BQATLAS_OIDC_ROLE_FILE=dev/oidc-roles.json
```

Restart the API. The login screen offers the external provider. Demo users are `admin`, `reader`, and `unassigned`, with password `RustOidc-Dev!2026`; only the explicit demo roles grant permissions. The provider administration login is `rust-dev-admin / RustKeycloak-Dev!2026`. This isolated realm is development/test data, with public credentials; it does not modify another application's provider.

Use discovery, Authorization Code + PKCE S256, state and nonce, verified issuer/audience/signature/expiry and access-token hash when supplied. Callback state is one-use and expires after ten minutes. Provider tokens stay in the server-side session. The actor ID is derived from issuer plus subject, not email. A session expires at the earlier of the ID token expiry and eight hours; refresh tokens are not used in this alpha.

Discovery/JWKS are refreshed on use after 60 seconds, with bounded HTTP requests and redirects disabled. Configure HTTPS issuer/endpoints/origin in Production. A key rollover may require the next cache refresh. Register `/signin-oidc` and `/signout-callback-oidc` under the application's public origin. Provider logout uses an ID token hint, an approved post-logout URI and a one-use state callback. A provider without an end-session endpoint performs local session logout.

`roles`, scalar `role`, Keycloak realm roles and this client's roles map through the explicit JSON role-to-permission file. Unknown roles/token permission claims grant nothing. The host rejects mapping entries that name undeclared permissions. Reader and unassigned users are tested at the API and manifest boundary.

OAuth2-only providers need an identity adapter or OIDC broker. API JWT bearer access is not implemented: any Authorization header on `/api/*` or `/odata/*` receives 401 and never falls back to browser cookies.

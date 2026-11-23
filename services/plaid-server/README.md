# Golden Plaid sync service

A small AWS Lambda that keeps Plaid's secret off the desktop app. The app sends its licence token
and a Plaid request; the service checks the licence (Ed25519, same key as the app, Plaid add-on paid
through today), adds the client id and secret, forwards to Plaid and returns the answer. It stores
nothing and never logs bodies.

| App path | Plaid endpoint |
| --- | --- |
| `POST /link/token` | `/link/token/create` |
| `POST /link/exchange` | `/item/public_token/exchange` |
| `POST /transactions/sync` | `/transactions/sync` |
| `POST /item/remove` | `/item/remove` |

## Deploy

1. Create a Lambda (Node 20+), upload `handler.mjs`, handler name `handler.handler`, with a Function URL (auth type NONE; the licence is the auth).
2. Set `PLAID_CLIENT_ID`, `PLAID_SECRET`, `PLAID_ENV` (`sandbox` first, then `production`), `GOLDEN_LICENCE_PUBLIC_KEY`. Keep the secret in Secrets Manager or Lambda's encrypted env vars.
3. Put the Function URL in the app build as `GOLDEN_PLAID_SERVICE_URL`.
4. Throttle the function (reserved concurrency) and set an AWS budget alarm.

Test: `npm test` (Node 20+, no dependencies).

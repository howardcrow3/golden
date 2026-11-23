#!/usr/bin/env bash
# Deploys the Plaid sync service to AWS Lambda (Node 22) behind a Function URL. Safe to re-run:
# it creates what is missing and updates the rest. Needs the AWS CLI with credentials, and
#   GOLDEN_LICENCE_PUBLIC_KEY   the public key the app is built with
# and optionally PLAID_CLIENT_ID, PLAID_SECRET (without them the service answers 503 until set),
# PLAID_ENV (sandbox or production, default sandbox).
set -euo pipefail

REGION=${AWS_REGION:-us-east-2}
FN=golden-plaid
ROLE=golden-plaid-lambda
HERE=$(cd "$(dirname "$0")" && pwd)
export AWS_DEFAULT_REGION=$REGION

: "${GOLDEN_LICENCE_PUBLIC_KEY:?GOLDEN_LICENCE_PUBLIC_KEY is required}"
PLAID_ENV=${PLAID_ENV:-sandbox}
case "$PLAID_ENV" in sandbox|production) ;; *) echo "PLAID_ENV must be sandbox or production" >&2; exit 1;; esac

echo "== Role =="
cat > /tmp/plaid-trust.json <<'JSON'
{"Version":"2012-10-17","Statement":[{"Effect":"Allow","Principal":{"Service":"lambda.amazonaws.com"},"Action":"sts:AssumeRole"}]}
JSON
if ! aws iam get-role --role-name "$ROLE" >/dev/null 2>&1; then
  aws iam create-role --role-name "$ROLE" --assume-role-policy-document file:///tmp/plaid-trust.json >/dev/null
  sleep 12
fi
aws iam attach-role-policy --role-name "$ROLE" --policy-arn arn:aws:iam::aws:policy/service-role/AWSLambdaBasicExecutionRole
ROLE_ARN=$(aws iam get-role --role-name "$ROLE" --query Role.Arn --output text)

echo "== Code =="
rm -f /tmp/golden-plaid.zip
(cd "$HERE" && zip -q -j /tmp/golden-plaid.zip handler.mjs package.json)

env_json() { # $1 = PLAID_REDIRECT_URI (may be empty)
  jq -n \
    --arg id "${PLAID_CLIENT_ID:-}" --arg secret "${PLAID_SECRET:-}" --arg env "$PLAID_ENV" \
    --arg key "$GOLDEN_LICENCE_PUBLIC_KEY" --arg redirect "$1" \
    '{Variables: ({PLAID_ENV: $env, GOLDEN_LICENCE_PUBLIC_KEY: ($key|gsub("\\s";"")), GOLDEN_APP_MAJOR: "1"}
       + (if $id != "" then {PLAID_CLIENT_ID: $id} else {} end)
       + (if $secret != "" then {PLAID_SECRET: $secret} else {} end)
       + (if $redirect != "" then {PLAID_REDIRECT_URI: $redirect} else {} end))}'
}

echo "== Function =="
if aws lambda get-function --function-name "$FN" >/dev/null 2>&1; then
  aws lambda update-function-code --function-name "$FN" --zip-file fileb:///tmp/golden-plaid.zip >/dev/null
  aws lambda wait function-updated --function-name "$FN"
else
  aws lambda create-function --function-name "$FN" --runtime nodejs22.x --handler handler.handler \
    --role "$ROLE_ARN" --zip-file fileb:///tmp/golden-plaid.zip --timeout 30 --memory-size 256 \
    --description "Golden bank sync: adds Plaid's secret and checks the licence" >/dev/null
  aws lambda wait function-active --function-name "$FN"
fi

echo "== Function URL =="
URL=$(aws lambda get-function-url-config --function-name "$FN" --query FunctionUrl --output text 2>/dev/null || true)
if [ -z "$URL" ] || [ "$URL" = "None" ]; then
  URL=$(aws lambda create-function-url-config --function-name "$FN" --auth-type NONE --query FunctionUrl --output text)
fi
URL=${URL%/}
aws lambda add-permission --function-name "$FN" --statement-id url-invoke --action lambda:InvokeFunctionUrl \
  --principal '*' --function-url-auth-type NONE >/dev/null 2>&1 || true
aws lambda add-permission --function-name "$FN" --statement-id url-invoke-fn --action lambda:InvokeFunction \
  --principal '*' --invoked-via-function-url >/dev/null 2>&1 || true

echo "== Settings =="
# The service's own /link page is the return address for banks that sign in through their app.
aws lambda update-function-configuration --function-name "$FN" --environment "$(env_json "$URL/link")" \
  --timeout 30 >/dev/null
aws lambda wait function-updated --function-name "$FN"
# Cap how many copies can run at once so a flood can't run up a bill. Skipped if the account has no spare.
aws lambda put-function-concurrency --function-name "$FN" --reserved-concurrent-executions 10 >/dev/null 2>&1 \
  || echo "note: could not reserve concurrency (account limit); continuing"

echo "== Smoke test =="
sleep 5
page=$(curl -s -o /dev/null -w '%{http_code}' "$URL/link" || true)
nolic=$(curl -s -o /dev/null -w '%{http_code}' -X POST -H 'content-type: application/json' -d '{}' "$URL/transactions/sync" || true)
echo "GET /link=$page (200 expected); POST /transactions/sync with no licence=$nolic (403 expected)"
echo "SERVICE_URL=$URL"
if [ -n "${GITHUB_OUTPUT:-}" ]; then echo "url=$URL" >> "$GITHUB_OUTPUT"; fi
if [ -n "${GITHUB_STEP_SUMMARY:-}" ]; then
  {
    echo "### Golden Plaid service"
    echo "URL: \`$URL\`  (Plaid environment: $PLAID_ENV)"
    echo "Smoke test: /link=$page, no-licence sync=$nolic"
    if [ -z "${PLAID_CLIENT_ID:-}" ] || [ -z "${PLAID_SECRET:-}" ]; then
      echo "**Plaid keys were not provided**: the service answers 503 until PLAID_CLIENT_ID and PLAID_SECRET are set and this is run again."
    fi
  } >> "$GITHUB_STEP_SUMMARY"
fi
[ "$page" = "200" ] && [ "$nolic" = "403" ]

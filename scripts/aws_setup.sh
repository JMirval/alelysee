#!/usr/bin/env bash
set -euo pipefail

# Alelysee AWS bootstrap (AWS CLI)
#
# Creates:
# - Cognito User Pool + App Client + Hosted UI domain (custom domain via Route53 + ACM)
# - S3 bucket for videos + CORS
# - CloudFront distribution for playback
#
# Writes generated values into ../.env (repo root).
#
# Prereqs:
# - aws CLI configured (AWS_ACCESS_KEY_ID/SECRET or SSO) and AWS_REGION set
# - jq installed
#
# Inputs (env vars):
# - APP_NAME (default: alelysee)
# - AWS_REGION (default: from aws config)
# - BASE_DOMAIN (default: alelysee.com)
# - AUTH_SUBDOMAIN (default: auth)  -> auth.alelysee.com
# - COGNITO_REDIRECT_URIS (optional; comma-separated)
#     default: http://localhost:8080/auth/callback,https://alelysee.com/auth/callback
# - S3_BUCKET (optional; default: ${APP_NAME}-videos-${ACCOUNT_ID}-${AWS_REGION})
#

need() { command -v "$1" >/dev/null 2>&1 || { echo "Missing dependency: $1" >&2; exit 1; }; }
need aws
need jq

APP_NAME="${APP_NAME:-alelysee}"
AWS_REGION="${AWS_REGION:-$(aws configure get region)}"
if [[ -z "${AWS_REGION}" ]]; then
  echo "AWS_REGION must be set (or configured via aws configure)" >&2
  exit 1
fi

BASE_DOMAIN="${BASE_DOMAIN:-alelysee.com}"
AUTH_SUBDOMAIN="${AUTH_SUBDOMAIN:-auth}"
AUTH_DOMAIN="${AUTH_SUBDOMAIN}.${BASE_DOMAIN}"
COGNITO_REDIRECT_URIS="${COGNITO_REDIRECT_URIS:-http://localhost:8080/auth/callback,https://${BASE_DOMAIN}/auth/callback}"
IFS=',' read -r -a CALLBACK_URLS <<< "${COGNITO_REDIRECT_URIS}"
LOGOUT_URLS=("http://localhost:8080/" "https://${BASE_DOMAIN}/")

ACCOUNT_ID="$(aws sts get-caller-identity --query Account --output text --region "${AWS_REGION}")"
S3_BUCKET="${S3_BUCKET:-${APP_NAME}-videos-${ACCOUNT_ID}-${AWS_REGION}}"

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
ENV_FILE="${ROOT_DIR}/.env"

echo "Using region: ${AWS_REGION}"
echo "Using app name: ${APP_NAME}"
echo "Using base domain: ${BASE_DOMAIN}"
echo "Using auth domain: ${AUTH_DOMAIN}"
echo "Using bucket: ${S3_BUCKET}"
echo "Writing env: ${ENV_FILE}"

tmp="$(mktemp -d)"
trap 'rm -rf "${tmp}"' EXIT

echo "== Cognito: create user pool =="
USER_POOL_ID="$(aws cognito-idp list-user-pools --region "${AWS_REGION}" --max-results 60 \
  | jq -r --arg n "${APP_NAME}-user-pool" '.UserPools[]? | select(.Name==$n) | .Id' | head -n 1)"
if [[ -z "${USER_POOL_ID}" ]]; then
  USER_POOL_ID="$(aws cognito-idp create-user-pool \
  --region "${AWS_REGION}" \
  --pool-name "${APP_NAME}-user-pool" \
  --auto-verified-attributes email \
  --username-attributes email \
  --policies 'PasswordPolicy={MinimumLength=10,RequireUppercase=true,RequireLowercase=true,RequireNumbers=true,RequireSymbols=false}' \
  --query 'UserPool.Id' --output text)"
fi

echo "USER_POOL_ID=${USER_POOL_ID}"

echo "== Cognito: create app client (Hosted UI) =="
APP_CLIENT_ID="$(aws cognito-idp list-user-pool-clients --region "${AWS_REGION}" --user-pool-id "${USER_POOL_ID}" --max-results 60 \
  | jq -r --arg n "${APP_NAME}-app-client" '.UserPoolClients[]? | select(.ClientName==$n) | .ClientId' | head -n 1)"
if [[ -z "${APP_CLIENT_ID}" ]]; then
  APP_CLIENT_ID="$(aws cognito-idp create-user-pool-client \
  --region "${AWS_REGION}" \
  --user-pool-id "${USER_POOL_ID}" \
  --client-name "${APP_NAME}-app-client" \
  --generate-secret false \
  --allowed-o-auth-flows-user-pool-client \
  --allowed-o-auth-flows implicit \
  --allowed-o-auth-scopes "openid" "email" "profile" \
  --callback-urls "${CALLBACK_URLS[@]}" \
  --logout-urls "${LOGOUT_URLS[@]}" \
  --supported-identity-providers "COGNITO" \
  --query 'UserPoolClient.ClientId' --output text)"
fi

echo "APP_CLIENT_ID=${APP_CLIENT_ID}"

echo "== Route53: find hosted zone for ${BASE_DOMAIN} =="
HOSTED_ZONE_ID="$(aws route53 list-hosted-zones-by-name --dns-name "${BASE_DOMAIN}." \
  | jq -r --arg n "${BASE_DOMAIN}." '.HostedZones[]? | select(.Name==$n) | .Id' | head -n 1 | sed 's|/hostedzone/||')"
if [[ -z "${HOSTED_ZONE_ID}" ]]; then
  echo "Could not find Route53 hosted zone for ${BASE_DOMAIN}. Create it first (or set BASE_DOMAIN/credentials to the right AWS account)." >&2
  exit 1
fi
echo "HOSTED_ZONE_ID=${HOSTED_ZONE_ID}"

echo "== ACM: request certificate for ${AUTH_DOMAIN} (DNS validation) =="
CERT_ARN="$(aws acm list-certificates --region "${AWS_REGION}" \
  | jq -r --arg d "${AUTH_DOMAIN}" '.CertificateSummaryList[]? | select(.DomainName==$d) | .CertificateArn' | head -n 1)"
if [[ -z "${CERT_ARN}" ]]; then
  CERT_ARN="$(aws acm request-certificate \
    --region "${AWS_REGION}" \
    --domain-name "${AUTH_DOMAIN}" \
    --validation-method DNS \
    --idempotency-token "${APP_NAME}$(date +%s)" \
    --query CertificateArn --output text)"
fi
echo "CERT_ARN=${CERT_ARN}"

echo "== ACM: create DNS validation record in Route53 =="
aws acm describe-certificate --region "${AWS_REGION}" --certificate-arn "${CERT_ARN}" > "${tmp}/cert.json"
VAL_NAME="$(jq -r '.Certificate.DomainValidationOptions[0].ResourceRecord.Name' "${tmp}/cert.json")"
VAL_TYPE="$(jq -r '.Certificate.DomainValidationOptions[0].ResourceRecord.Type' "${tmp}/cert.json")"
VAL_VALUE="$(jq -r '.Certificate.DomainValidationOptions[0].ResourceRecord.Value' "${tmp}/cert.json")"

cat > "${tmp}/rrset.json" <<EOF
{
  "Comment": "ACM validation for ${AUTH_DOMAIN}",
  "Changes": [
    {
      "Action": "UPSERT",
      "ResourceRecordSet": {
        "Name": "${VAL_NAME}",
        "Type": "${VAL_TYPE}",
        "TTL": 300,
        "ResourceRecords": [{ "Value": "${VAL_VALUE}" }]
      }
    }
  ]
}
EOF
aws route53 change-resource-record-sets --hosted-zone-id "${HOSTED_ZONE_ID}" --change-batch "file://${tmp}/rrset.json" >/dev/null

echo "== ACM: wait for certificate validation (can take a few minutes) =="
aws acm wait certificate-validated --region "${AWS_REGION}" --certificate-arn "${CERT_ARN}"

echo "== Cognito: set custom domain ${AUTH_DOMAIN} =="
DOMAIN_STATUS="$(aws cognito-idp describe-user-pool-domain --region "${AWS_REGION}" --domain "${AUTH_DOMAIN}" 2>/dev/null | jq -r '.DomainDescription.Domain // empty' || true)"
if [[ -z "${DOMAIN_STATUS}" ]]; then
  aws cognito-idp create-user-pool-domain \
    --region "${AWS_REGION}" \
    --user-pool-id "${USER_POOL_ID}" \
    --domain "${AUTH_DOMAIN}" \
    --custom-domain-config "CertificateArn=${CERT_ARN}" >/dev/null
fi

COGNITO_DOMAIN="https://${AUTH_DOMAIN}"
echo "COGNITO_DOMAIN=${COGNITO_DOMAIN}"

echo "== Cognito: fetch CloudFront distribution domain for custom domain =="
CF_COGNITO_DOMAIN="$(aws cognito-idp describe-user-pool-domain --region "${AWS_REGION}" --domain "${AUTH_DOMAIN}" \
  | jq -r '.DomainDescription.CloudFrontDistribution')"
if [[ -z "${CF_COGNITO_DOMAIN}" || "${CF_COGNITO_DOMAIN}" == "null" ]]; then
  echo "Cognito domain exists but CloudFrontDistribution not ready yet. Re-run script in a few minutes." >&2
  exit 1
fi

echo "== Route53: create CNAME auth domain -> Cognito CloudFront =="
cat > "${tmp}/auth_cname.json" <<EOF
{
  "Comment": "Cognito custom domain for ${AUTH_DOMAIN}",
  "Changes": [
    {
      "Action": "UPSERT",
      "ResourceRecordSet": {
        "Name": "${AUTH_DOMAIN}.",
        "Type": "CNAME",
        "TTL": 300,
        "ResourceRecords": [{ "Value": "${CF_COGNITO_DOMAIN}" }]
      }
    }
  ]
}
EOF
aws route53 change-resource-record-sets --hosted-zone-id "${HOSTED_ZONE_ID}" --change-batch "file://${tmp}/auth_cname.json" >/dev/null

echo "== S3: create bucket (videos) =="
if ! aws s3api head-bucket --bucket "${S3_BUCKET}" --region "${AWS_REGION}" >/dev/null 2>&1; then
  if [[ "${AWS_REGION}" == "us-east-1" ]]; then
    aws s3api create-bucket --bucket "${S3_BUCKET}" --region "${AWS_REGION}" >/dev/null
  else
    aws s3api create-bucket --bucket "${S3_BUCKET}" --region "${AWS_REGION}" \
      --create-bucket-configuration LocationConstraint="${AWS_REGION}" >/dev/null
  fi
fi

echo "== S3: set CORS =="
cat > "${tmp}/cors.json" <<EOF
{
  "CORSRules": [
    {
      "AllowedHeaders": ["*"],
      "AllowedMethods": ["PUT", "GET", "HEAD"],
      "AllowedOrigins": ["http://localhost:8080", "https://${BASE_DOMAIN}"],
      "ExposeHeaders": [],
      "MaxAgeSeconds": 3000
    }
  ]
}
EOF
aws s3api put-bucket-cors --bucket "${S3_BUCKET}" --cors-configuration "file://${tmp}/cors.json" --region "${AWS_REGION}" >/dev/null

echo "== CloudFront: create distribution for S3 playback =="
# Note: This uses the S3 REST endpoint as an origin. For production you may want an Origin Access Control.
ORIGIN_DOMAIN="${S3_BUCKET}.s3.${AWS_REGION}.amazonaws.com"
CALLER_REF="${APP_NAME}-$(date +%s)"

cat > "${tmp}/cf.json" <<EOF
{
  "CallerReference": "${CALLER_REF}",
  "Comment": "${APP_NAME} videos",
  "Enabled": true,
  "Origins": {
    "Quantity": 1,
    "Items": [
      {
        "Id": "s3origin",
        "DomainName": "${ORIGIN_DOMAIN}",
        "S3OriginConfig": { "OriginAccessIdentity": "" }
      }
    ]
  },
  "DefaultCacheBehavior": {
    "TargetOriginId": "s3origin",
    "ViewerProtocolPolicy": "redirect-to-https",
    "AllowedMethods": { "Quantity": 2, "Items": ["GET", "HEAD"], "CachedMethods": { "Quantity": 2, "Items": ["GET", "HEAD"] } },
    "Compress": true,
    "ForwardedValues": { "QueryString": false, "Cookies": { "Forward": "none" } },
    "MinTTL": 0
  },
  "DefaultRootObject": ""
}
EOF

CF_OUT="$(aws cloudfront create-distribution --distribution-config "file://${tmp}/cf.json")"
CF_DOMAIN="$(echo "${CF_OUT}" | jq -r '.Distribution.DomainName')"

echo "CLOUDFRONT_DOMAIN=${CF_DOMAIN}"

echo "== Write .env (contains secrets; do not commit) =="
cat > "${ENV_FILE}" <<EOF
DATABASE_URL=
AWS_REGION=${AWS_REGION}

COGNITO_REGION=${AWS_REGION}
COGNITO_USER_POOL_ID=${USER_POOL_ID}
COGNITO_APP_CLIENT_ID=${APP_CLIENT_ID}
COGNITO_DOMAIN=${COGNITO_DOMAIN}
COGNITO_REDIRECT_URI=${CALLBACK_URLS[0]}

S3_BUCKET=${S3_BUCKET}
CLOUDFRONT_BASE_URL=https://${CF_DOMAIN}
EOF

echo "Done."
echo "Next:"
echo "- Ensure auth DNS has propagated: https://${AUTH_DOMAIN}"
echo "- Fill DATABASE_URL (RDS or local Postgres)"
echo "- Run: cd packages/web && dx serve -p web --web --fullstack"



## AWS CLI bootstrap

This repo includes a helper script to create the core AWS resources needed by the app:
- Cognito User Pool + Hosted UI domain
- S3 bucket for video uploads (+ CORS)
- CloudFront distribution for video playback

### Prerequisites
- AWS CLI configured (`aws sts get-caller-identity` works)
- `jq` installed

### Run

From repo root:

```bash
chmod +x scripts/aws_setup.sh
APP_NAME=heliastes \
AWS_REGION=us-east-1 \
COGNITO_DOMAIN_PREFIX=heliastes-dev-1234 \
COGNITO_REDIRECT_URI=http://localhost:8080/auth/callback \
./scripts/aws_setup.sh
```

This writes **`.env`** at the repo root. `.env` is gitignored.

### Notes
- OAuth providers (Google/Apple) require extra setup and client IDs/secrets; the script currently creates a Cognito-only Hosted UI client for MVP.
- For production you’ll likely want an Origin Access Control (OAC) for CloudFront → S3 and stricter S3 bucket policies.



## Alelysee AWS setup (Cognito + S3 + CloudFront)

This provisions the minimum AWS resources needed by the current app code:
- **Cognito User Pool** with Hosted UI using **custom domain** `auth.alelysee.com`
- **S3** bucket for video uploads + CORS
- **CloudFront** distribution for video playback
- Writes a repo-root **`.env`** (gitignored)

### Prerequisites
- Your domain **`alelysee.com`** is hosted in **Route53** in the same AWS account you’re running the CLI in.
- `aws` CLI configured and `jq` installed.

### Run

```bash
cd /Users/dode/Documents/rust/heliastes
chmod +x scripts/aws_setup.sh

APP_NAME=alelysee \
AWS_REGION=us-east-1 \
BASE_DOMAIN=alelysee.com \
AUTH_SUBDOMAIN=auth \
COGNITO_REDIRECT_URIS="http://localhost:8080/auth/callback,https://alelysee.com/auth/callback" \
./scripts/aws_setup.sh
```

After it completes:
- `.env` will contain `COGNITO_*`, `S3_BUCKET`, `CLOUDFRONT_BASE_URL`
- You must set `DATABASE_URL` yourself (RDS or local)

### Next steps (still manual)
- If you want Google/Apple OAuth, you must create the provider apps and then create Cognito identity providers.
- If you want AWS-hosted backend (ECS/App Runner) + RDS provisioning, we can add additional scripts; it’s feasible but needs choices (VPC, instance sizes, public vs private subnets, etc.).



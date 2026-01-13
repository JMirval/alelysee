# AWS Deployment (Alelysee)

This project is a Dioxus 0.7 **fullstack** workspace (web/desktop/mobile clients + shared `api` server functions). For production we host the server on AWS and use RDS (Postgres), Cognito (auth), and S3 + CloudFront (videos). Deployment is managed via CloudFormation and GitHub Actions (see `AWS_RUNBOOK.md`).

## Components
- **Compute**: ECS Fargate service behind an ALB
- **Database**: RDS Postgres
- **Auth**: Cognito User Pool + Hosted UI (email/password + OAuth providers)
- **Uploads**: S3 (pre-signed PUT)
- **Playback**: CloudFront distribution pointing to S3 origin
- **Secrets/Config**: Secrets Manager / SSM Parameter Store + task env vars

## Required environment variables
Copy `[env.example](/Users/dode/Documents/rust/alelysee/env.example)` into your runtime environment (Secrets Manager recommended).

- `DATABASE_URL` (RDS)
- `COGNITO_REGION`
- `COGNITO_USER_POOL_ID`
- `COGNITO_APP_CLIENT_ID`
- `COGNITO_DOMAIN`
- `COGNITO_REDIRECT_URI` (production example: `https://your-domain.com/auth/callback`)
- `S3_BUCKET`
- `CLOUDFRONT_BASE_URL` (recommended for video playback)
- Standard AWS SDK env vars / IAM role permissions (ECS task role preferred)

## AWS setup steps (high level)

### 1) RDS Postgres
- Create an RDS Postgres instance.
- Put it in private subnets and allow inbound from the ECS service security group.
- Create a DB + user and set `DATABASE_URL`.
- Note: the server auto-runs SQL migrations on boot (see `packages/api/migrations`).

### 2) Cognito User Pool
- Create a User Pool.
- Enable **email/password** sign-up + verification.
- Add OAuth providers (Google/Apple/etc) if desired.
- Create an App Client and enable **Hosted UI**.
- Add callback URLs:
   - `http://localhost:8080/auth/callback` (dev)
   - `https://your-domain.com/auth/callback` (prod)
- Set:
   - `COGNITO_DOMAIN` to your hosted UI domain
   - `COGNITO_APP_CLIENT_ID` to the app client id
   - `COGNITO_REGION` + `COGNITO_USER_POOL_ID` accordingly

The app uses an OAuth **implicit flow** redirect for MVP (token returned in URL fragment). You can migrate to Authorization Code + PKCE later.

### 3) S3 + CORS for uploads
- Create an S3 bucket for videos.
- Configure CORS to allow browser PUT to pre-signed URLs from your app origin.
- Example CORS policy (adjust origins):

```json
[
  {
    "AllowedHeaders": ["*"],
    "AllowedMethods": ["PUT", "GET", "HEAD"],
    "AllowedOrigins": ["http://localhost:8080", "https://your-domain.com"],
    "ExposeHeaders": []
  }
]
```

### 4) CloudFront
- Create a CloudFront distribution with the S3 bucket as origin.
- Set `CLOUDFRONT_BASE_URL` to the distribution base URL.

### 5) ECS Fargate + ALB
- Deployment is handled by CloudFormation.
- Configure an ALB listener (HTTPS recommended) and target group to your ECS service.
- Ensure the ECS task role allows:
  - `s3:PutObject`, `s3:GetObject`, `s3:HeadObject` on the video bucket

## Local dev
From `packages/web`:

```bash
dx serve
```

Make sure the environment variables above are set in the shell you run `dx serve` from.

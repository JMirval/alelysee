# AWS CloudFormation + GitHub Actions Runbook

This repo now deploys via CloudFormation and a GitHub Actions workflow. The workflow builds the Docker image, pushes to ECR, and deploys the ECS/RDS/ALB stack.

## 1) Prereqs
- AWS account with permissions to create VPC, ECS, ALB, RDS, IAM, CloudWatch, and ECR.
- Cognito User Pool + Hosted UI domain already created.
- S3 bucket for uploads (and optional CloudFront distribution for playback).
- GitHub OIDC role for Actions (recommended).

## 2) CloudFormation templates
- `infra/ecr.yml` creates the ECR repository.
- `infra/app.yml` creates VPC, subnets, ALB, ECS Fargate service, and (optionally) RDS.

## 3) GitHub Actions workflow
- `/.github/workflows/deploy.yml`
- Runs on `main` push or manual dispatch.
- Creates/updates ECR stack → builds/pushes image → deploys app stack.

## 4) GitHub Secrets (required)
These secrets are referenced by the workflow:

- `AWS_ROLE_ARN` (OIDC role to assume)
- `AWS_REGION`

- `COGNITO_REGION`
- `COGNITO_USER_POOL_ID`
- `COGNITO_APP_CLIENT_ID`
- `COGNITO_DOMAIN`
- `COGNITO_REDIRECT_URI`

- `S3_BUCKET`
- `CLOUDFRONT_BASE_URL` (empty string is allowed)

Database (choose one path):

**A) CloudFormation‑managed RDS (default)**
- `DB_USERNAME`
- `DB_PASSWORD`
- Optional: `DB_NAME` (default: `heliastes`)
- Optional: `DB_INSTANCE_CLASS` (default: `db.t4g.micro`)
- Optional: `DB_ALLOCATED_STORAGE` (default: `20`)
- Leave `DATABASE_URL` empty.

**B) External database**
- `DATABASE_URL` (full URL, e.g. `postgres://user:pass@host:5432/db`)
- `DB_PASSWORD` is still required by the workflow; set any value.

## 5) First deploy
1. Push to `main` (or run workflow_dispatch). The workflow will:
   - Deploy the ECR stack.
   - Build and push the Docker image.
   - Deploy the app stack with the new image.
2. Grab the ALB DNS from CloudFormation outputs and set your app DNS record.

## 6) Useful CLI checks
```
aws cloudformation describe-stacks --stack-name heliastes-prod-app \
  --query "Stacks[0].Outputs" --output table
```

## 7) Notes
- The ALB is HTTP only. Add HTTPS + ACM later if needed.
- RDS instances use `DeletionPolicy: Snapshot` to protect data on stack deletion.
- The ECS tasks run with a public IP; tighten later if you add NAT + private subnets.

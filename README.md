# Alelysee

Cross-platform **fullstack** Dioxus 0.7 app for political proposals and programs:
- **Auth**: AWS Cognito (email/password + OAuth via Hosted UI)
- **Profiles**: create/edit profile
- **Content**: proposals + programs (bundle proposals)
- **Interaction**: up/down votes, comments, and an activity feed on your profile
- **Video**: upload videos for proposals/programs to S3 via pre-signed URLs; play via CloudFront

## Quick Start

### Prerequisites
- Rust 1.91.1+
- AWS CLI configured
- Docker (optional, for containerized deployment)
- PostgreSQL (for local development)

### Setup and Development

1. **Clone and setup:**
   ```bash
   git clone <repository-url>
   cd alelysee
   make setup
   ```

2. **Configure environment:**
   ```bash
   cp env.example .env
   # Edit .env with your configuration
   ```

3. **Setup AWS resources:**
   ```bash
   export CERT_ARN="arn:aws:acm:region:account:certificate/certificate-id"
   make aws-setup
   ```

4. **Start development server:**
   ```bash
   make dev
   ```

## Repo Layout
- `packages/web`: web client + fullstack server build
- `packages/desktop`: desktop client (webview)
- `packages/mobile`: mobile client
- `packages/ui`: shared UI/components (responsive CSS)
- `packages/api`: shared server functions (DB/auth/uploads)
- `scripts/`: deployment and setup scripts
- `Dockerfile`: container build configuration
- `docker-compose.yml`: local development stack

## Development Commands

Use the Makefile for common operations:

```bash
make help           # Show all available commands
make dev            # Start development server
make test           # Run all tests
make build          # Build all packages
make clean          # Clean build artifacts
make fmt            # Format code
make lint           # Run clippy linter
```

### Local Development with Docker

```bash
# Start full stack (PostgreSQL + App)
docker-compose up

# Or build and run manually
make docker-build
make docker-run
```

## Key Routes
- `/auth/signin` and `/auth/callback`
- `/me` and `/me/edit`
- `/proposals`, `/proposals/new`, `/proposals/:id`
- `/programs`, `/programs/new`, `/programs/:id`
- `/api/health` - health check endpoint

## AWS Deployment

### Automated Deployment

1. **Setup AWS resources:**
   ```bash
   export CERT_ARN="your-certificate-arn"
   export DOCKER_REGISTRY="your-registry"
   make aws-setup
   ```

2. **Deploy to environment:**
   ```bash
   make deploy-dev     # Deploy to development
   make deploy-prod    # Deploy to production
   ```

### Manual Deployment

See `[AWS_DEPLOYMENT.md](AWS_DEPLOYMENT.md)` for detailed AWS setup instructions.

### Infrastructure Components

- **ECS Fargate**: Containerized application
- **RDS PostgreSQL**: Database
- **ALB**: Load balancer with health checks
- **Cognito**: Authentication
- **S3 + CloudFront**: Video storage and delivery
- **VPC**: Networking with public/private subnets

## Monitoring & Analytics

### Health Checks & Metrics

The application provides several monitoring endpoints:

- **`/api/health`** - Basic health check (returns "OK")
- **`/api/health/detailed`** - Detailed health with JSON response
- **`/api/metrics`** - Prometheus-style metrics for monitoring

### AWS Resource Management

```bash
# View all AWS resources created by deployment
make aws-resources

# View ALB DNS names for DNS configuration
make aws-alb-dns

# Clean up unused resources (safe - only empty VPCs)
make aws-cleanup-auto

# Interactive cleanup with confirmation prompts
make aws-cleanup-unused

# DANGER: Delete ALL alelysee resources (VPCs, ALBs, ECS, RDS, etc.)
make aws-cleanup-force
```

### AWS Infrastructure Setup

Deployment is handled via CloudFormation + GitHub Actions. See `AWS_RUNBOOK.md` for required secrets and workflow behavior.

#### 3. Monitoring Setup (requires deployed ALB)
```bash
# Setup monitoring (requires deployed ALB)
make aws-monitoring ENVIRONMENT=prod

# Check monitoring status
make aws-monitoring-status
```

Includes:
- **CloudWatch Dashboard** - Real-time metrics visualization
- **CloudWatch Logs** - Centralized application logging
- **Route 53 Health Checks** - Endpoint monitoring
- **CloudWatch Alarms** - Automated alerting
- **AWS X-Ray** - Application tracing

### Monitoring URLs

After deployment, monitoring is available at:
- **Dashboard**: `https://<region>.console.aws.amazon.com/cloudwatch/home#dashboards:name=<app>-<env>-dashboard`
- **Logs**: `https://<region>.console.aws.amazon.com/cloudwatch/home#logsV2:log-groups`
- **Health Checks**: `https://<region>.console.aws.amazon.com/route53/healthchecks/home`

## CI/CD

GitHub Actions builds the image, pushes to ECR, and deploys the CloudFormation stack on `main`. See `AWS_RUNBOOK.md` for details.

This will trigger the production deployment pipeline.

## Environment Variables

See `[env.example](env.example)` for required environment variables.

Key variables:
- `DATABASE_URL`: PostgreSQL connection string
- `AWS_REGION`: AWS region
- `COGNITO_*`: Cognito configuration
- `S3_BUCKET`: S3 bucket for uploads
- `CLOUDFRONT_BASE_URL`: CloudFront distribution URL

## Troubleshooting

### Common Issues

1. **Build fails**: Ensure Rust 1.91.1+ and run `make setup`
2. **AWS setup fails**: Check AWS CLI configuration and permissions
3. **Database connection fails**: Verify DATABASE_URL and ensure RDS is accessible
4. **Health check fails**: Check application logs and environment variables

### Useful Commands

```bash
make health-check    # Test application health
make logs            # View application logs
make env-check       # Validate environment variables
make aws-status      # Check AWS resources status
```

## Contributing

1. Fork the repository
2. Create a feature branch
3. Make changes and add tests
4. Run `make test && make lint`
5. Submit a pull request

## License

[Add license information here]
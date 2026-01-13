# Alelysee

Cross-platform **fullstack** Dioxus 0.7 app for political proposals and programs:
- **Auth**: Hosted UI (email/password + OAuth)
- **Profiles**: create/edit profile
- **Content**: proposals + programs (bundle proposals)
- **Interaction**: up/down votes, comments, and an activity feed on your profile
- **Video**: upload videos for proposals/programs via pre-signed URLs; play via CDN

## Quick Start

### Prerequisites
- Rust 1.91.1+
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

3. **Start development server:**
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

## Monitoring & Analytics

### Health Checks & Metrics

The application provides several monitoring endpoints:

- **`/api/health`** - Basic health check (returns "OK")
- **`/api/health/detailed`** - Detailed health with JSON response
- **`/api/metrics`** - Prometheus-style metrics for monitoring

## CI/CD

GitHub Actions deploys to Railway for both dev and prod.

## Environment Variables

See `[env.example](env.example)` for required environment variables.

Key variables:
- `DATABASE_URL`: PostgreSQL connection string
- `AUTH_*`: Auth configuration
- `STORAGE_BUCKET`: Object storage bucket for uploads
- `MEDIA_BASE_URL`: CDN base URL

## Troubleshooting

### Common Issues

1. **Build fails**: Ensure Rust 1.91.1+ and run `make setup`
2. **Database connection fails**: Verify DATABASE_URL and ensure Postgres is accessible
3. **Health check fails**: Check application logs and environment variables

### Useful Commands

```bash
make health-check    # Test application health
make logs            # View application logs
make env-check       # Validate environment variables
```

## Contributing

1. Fork the repository
2. Create a feature branch
3. Make changes and add tests
4. Run `make test && make lint`
5. Submit a pull request

## License

[Add license information here]

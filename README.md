# Heliastes

Cross-platform **fullstack** Dioxus 0.7 app for political proposals and programs:
- **Auth**: AWS Cognito (email/password + OAuth via Hosted UI)
- **Profiles**: create/edit profile
- **Content**: proposals + programs (bundle proposals)
- **Interaction**: up/down votes, comments, and an activity feed on your profile
- **Video**: upload videos for proposals/programs to S3 via pre-signed URLs; play via CloudFront

## Repo layout
- `packages/web`: web client + fullstack server build
- `packages/desktop`: desktop client (webview)
- `packages/mobile`: mobile client
- `packages/ui`: shared UI/components (responsive CSS)
- `packages/api`: shared server functions (DB/auth/uploads)

## Local development

1) Set environment variables (see `[env.example](/Users/dode/Documents/rust/heliastes/env.example)`).

2) Run from one of the platform crates:

```bash
cd packages/web
dx serve
```

You can also run desktop/mobile:

```bash
cd packages/desktop
dx serve
```

```bash
cd packages/mobile
dx serve --platform android
```

## Key routes
- `/auth/signin` and `/auth/callback`
- `/me` and `/me/edit`
- `/proposals`, `/proposals/new`, `/proposals/:id`
- `/programs`, `/programs/new`, `/programs/:id`

## AWS deployment
See `[AWS_DEPLOYMENT.md](/Users/dode/Documents/rust/heliastes/AWS_DEPLOYMENT.md)`.


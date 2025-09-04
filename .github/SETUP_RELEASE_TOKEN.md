# Setting Up Release Token

For the release automation to create PRs, you need to set up a Personal Access Token (PAT) or use a GitHub App.

## Option 1: Personal Access Token (Recommended for Personal Projects)

1. Go to GitHub Settings → Developer settings → Personal access tokens → Tokens (classic)
2. Click "Generate new token (classic)"
3. Give it a name like "release-plz-token"
4. Select scopes:
   - `repo` (full control)
   - `workflow` (update workflows)
5. Generate and copy the token
6. In your repository: Settings → Secrets and variables → Actions
7. Create a new secret named `RELEASE_PLZ_TOKEN` with the token value

## Option 2: GitHub App (Recommended for Organizations)

1. Create a GitHub App with:
   - Contents: Read & Write
   - Pull requests: Read & Write
   - Actions: Read
2. Install the app on your repository
3. Use the app's token in the workflow

## Option 3: Use Default GITHUB_TOKEN (Limited)

The default `GITHUB_TOKEN` works but with limitations:
- Pull requests created by workflows DO trigger GitHub-native features like Copilot Reviews
- Some cross-repo operations may fail
- Push/tag events from workflows won't trigger other workflows (by design)

To use this option, no additional setup needed, but be aware of limitations.

## Testing

After setup, push a commit to main:
```bash
git commit -m "feat: test release automation"
git push origin main
```

Check Actions tab for the workflow run and look for a new "Release" PR.

# Publish the interface prototype

The public prototype is at [combe.pages.dev](https://combe.pages.dev/).

Cloudflare Pages project: `combe`, account: `samzong`, production branch: `main`. The project uses Direct Upload; pushing Git commits does not update the site.

## Update the site

Run from the repository root with Wrangler 4 installed and authenticated to the `samzong` account. Use `wrangler whoami` to check access, or `wrangler login` to sign in.

```sh
mkdir -p .local
COMBE_PAGES_DIR="$(mktemp -d "$PWD/.local/pages-combe.XXXXXX")"
cp docs/design.html "$COMBE_PAGES_DIR/index.html"
cp docs/design.html docs/design.spacing.js docs/spacing.md docs/radii.md docs/DESIGN.md "$COMBE_PAGES_DIR/"
CLOUDFLARE_ACCOUNT_ID=277fa0738593e7eed6348ef98d5a1e1d wrangler --cwd .local pages deploy "$COMBE_PAGES_DIR" --project-name combe --branch main
```

The upload contains the prototype, its script, and its three linked reference documents. `index.html` is a deployment copy of `docs/design.html`; keep editing the original. Add any new local dependencies to the copy command when the prototype starts using them. Build folders and Wrangler cache stay under `.local/`.

After deployment, open [combe.pages.dev](https://combe.pages.dev/) and check the appearance selector, component selector, and sidebar interactions. Wrangler also prints a URL for that specific deployment.

Direct Upload cannot be switched to Cloudflare's Git integration in place. Future automation can run the same Wrangler upload command from CI. See the [Cloudflare Direct Upload guide](https://developers.cloudflare.com/pages/get-started/direct-upload/).

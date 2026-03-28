# yamalgam Documentation Site

Built with [Astro Starlight](https://starlight.astro.build/).

Content is sourced from the `docs/` directory at the project root.

## Development

```bash
# Install dependencies
npm install

# Start dev server (http://localhost:4321)
npm run dev

# Build for production
npm run build

# Preview production build
npm run preview
```

## Deployment

This site deploys to **Cloudflare Pages** via git integration.

Connect this repository in the [Cloudflare dashboard](https://dash.cloudflare.com/):

1. Go to **Workers & Pages** > **Create** > **Pages**
2. Connect your GitHub repository
3. Set build command: `cd site && npm run build`
4. Set build output directory: `site/dist`


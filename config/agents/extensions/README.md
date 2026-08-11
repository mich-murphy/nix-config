# Pi extensions

This directory is the live source for global Pi coding-agent extensions. Home
Manager links it to `~/.pi/agent/extensions`, matching the shared skills setup.
Pi discovers top-level TypeScript files and subdirectories containing an
`index.ts` entry point.

Keep source, tests, package manifests, and lockfiles here. Do not commit
`node_modules` or credentials. Extensions with npm dependencies need a local
install after a fresh checkout or lockfile change:

```sh
for extension in config/agents/extensions/*/package-lock.json; do
  npm ci --prefix "$(dirname "$extension")"
done
```

The Home Manager declaration forcibly replaces the previously unmanaged
extension directory on its first activation. Confirm every wanted extension is
present here before switching the profile.

Use Pi's `/reload` command after editing extension source. Home Manager only
needs to be rebuilt when the link declaration changes or the checkout moves.

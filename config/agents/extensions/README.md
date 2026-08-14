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

Before the first Home Manager activation, preserve any previously unmanaged
extension directory and then switch the profile:

```sh
mv ~/.pi/agent/extensions ~/.pi/agent/extensions.pre-home-manager
home-manager switch --flake '.#michael@ai-dev'
```

Confirm every wanted extension is present here before migrating. The Home
Manager `force` option can replace files and symbolic links, but not a real
directory. Keep the preserved directory until the managed extensions have been
verified.

Use Pi's `/reload` command after editing extension source. Home Manager only
needs to be rebuilt when the link declaration changes or the checkout moves.

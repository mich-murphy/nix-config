# Pi extensions

This directory contains the source for global Pi coding-agent extensions. Home
Manager packages extensions with npm dependencies and links the resulting
immutable directory to `~/.pi/agent/extensions`. Pi discovers top-level
TypeScript files and subdirectories containing an `index.ts` entry point.

Keep source, tests, package manifests, and lockfiles here. Do not commit
`node_modules` or credentials. Each packaged extension pins its npm dependency
closure with `npmDepsHash` in `home/coding-agents.nix`. Update that hash when its
lockfile changes.

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

Rebuild and activate Home Manager after changing extension source or lockfiles,
then restart Pi or use its `/reload` command.

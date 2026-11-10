# `nix run .#deploy` and `nix run .#status`. fleet.sh runs on the Mac and
# fleet-host.sh on each host; see their headers.
{
  pkgs,
  lib,
  # [ { name; kind = "darwin" | "home"; config; checkout; skills; } ]
  hosts,
}:
let
  # Sent to other hosts over SSH, so it uses each host's own tools.
  host = pkgs.writeShellApplication {
    name = "fleet-host";
    text = builtins.readFile ./fleet-host.sh;
  };
  hostsFile = pkgs.writeText "fleet-hosts" (
    lib.concatMapStrings (
      h:
      lib.concatStringsSep "\t" [
        h.name
        h.kind
        h.config
        h.checkout
        h.skills
      ]
      + "\n"
    ) hosts
  );
  app =
    action:
    pkgs.writeShellApplication {
      name = action;
      runtimeEnv = {
        FLEET_ACTION = action;
        FLEET_HOSTS = hostsFile;
        FLEET_HOST_SCRIPT = lib.getExe host;
      };
      text = builtins.readFile ./fleet.sh;
    };
in
{
  inherit host;
  deploy = app "deploy";
  status = app "status";
}

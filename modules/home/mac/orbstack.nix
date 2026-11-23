{
  config,
  lib,
  pkgs,
  ...
}:
{
  home.activation.orbstackSettings = lib.hm.dag.entryAfter [ "linkGeneration" ] (
    lib.hm.generators.mkImpureConfigMerger {
      inherit pkgs;
      format = "json";
      empty = "{}";
      mode = "600";
      path = "${config.home.homeDirectory}/.orbstack/vmconfig.json";
      staticSettings = (pkgs.formats.json { }).generate "orbstack-vmconfig.json" {
        "docker.expose_ports_to_lan" = false;
        "machines.expose_ports_to_lan" = false;
      };
      jqOperation = "$dynamic * $static";
    }
  );
}

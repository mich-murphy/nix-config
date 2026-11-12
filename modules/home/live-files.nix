{
  config,
  inputs,
  lib,
  ...
}:
{
  options.nixConfig.checkout = lib.mkOption {
    type = lib.types.nullOr lib.types.str;
    default = "${config.home.homeDirectory}/dev/nix-config";
    description = ''
      Absolute path of this repository's checkout. Writable configuration is
      linked here with mkOutOfStoreSymlink, so applications can save to it.
      Null on a host with no checkout: this repository's files then come from
      the deployed generation and are read-only.
    '';
  };

  options.nixConfig.skills = lib.mkOption {
    type = lib.types.str;
    default = "${config.home.homeDirectory}/dev/skills";
    description = ''
      Absolute path of the mich-murphy/skills checkout, linked live into each
      coding agent's skills directory.
    '';
  };

  config.lib.nixConfig.file =
    path:
    if config.nixConfig.checkout == null then
      inputs.self + "/${path}"
    else
      config.lib.file.mkOutOfStoreSymlink "${config.nixConfig.checkout}/${path}";
}

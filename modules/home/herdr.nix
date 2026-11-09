{
  config,
  pkgs,
  ...
}:
{
  # From nixpkgs on every host, so flake.lock keeps the client and any remote
  # server on the same version.
  home.packages = [ pkgs.herdr ];

  xdg.configFile."herdr/config.toml".source =
    config.lib.file.mkOutOfStoreSymlink "${config.nixConfig.checkout}/config/herdr/config.toml";
}

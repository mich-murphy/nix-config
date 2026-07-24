{
  config,
  pkgs,
  repoRoot,
  ...
}: {
  home.packages = [pkgs.herdr];
  xdg.configFile."herdr/config.toml".source =
    config.lib.file.mkOutOfStoreSymlink "${repoRoot}/config/herdr/config.toml";
}

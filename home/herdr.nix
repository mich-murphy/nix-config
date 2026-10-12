{
  config,
  repoRoot,
  ...
}: {
  xdg.configFile."herdr/config.toml".source =
    config.lib.file.mkOutOfStoreSymlink "${repoRoot}/config/herdr/config.toml";
}

{
  config,
  ...
}:
{
  xdg.configFile."herdr/config.toml".source =
    config.lib.file.mkOutOfStoreSymlink "${config.nixConfig.checkout}/config/herdr/config.toml";
}

{
  config,
  pkgs,
  ...
}:
{
  home.packages = [ pkgs.herdr ];

  xdg.configFile."herdr/config.toml".source = config.lib.nixConfig.file "config/herdr/config.toml";
}

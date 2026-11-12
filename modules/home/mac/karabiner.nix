{
  config,
  ...
}:
{
  xdg.configFile."karabiner".source = config.lib.nixConfig.file "config/karabiner";
}

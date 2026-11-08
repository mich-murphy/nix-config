{
  config,
  ...
}:
{
  # Karabiner watches the directory itself. Linking karabiner.json directly
  # prevents its FSEvents-based reload detection from seeing GUI writes.
  xdg.configFile."karabiner".source =
    config.lib.file.mkOutOfStoreSymlink "${config.nixConfig.checkout}/config/karabiner";
}

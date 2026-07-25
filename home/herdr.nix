{
  config,
  lib,
  repoRoot,
  ...
}: {
  xdg.configFile."herdr/config.toml".source =
    config.lib.file.mkOutOfStoreSymlink "${repoRoot}/config/herdr/config.toml";

  # Herdr clients detach without stopping their persistent server. Reload each
  # live server after Home Manager replaces the config symlink.
  home.activation.reloadHerdrConfig = lib.hm.dag.entryAfter ["linkGeneration"] ''
    reload_herdr_config() {
      local socket="$1"

      if [[ -S "$socket" ]]; then
        $DRY_RUN_CMD env HERDR_SOCKET_PATH="$socket" \
          /opt/homebrew/bin/herdr server reload-config >/dev/null || true
      fi
    }

    reload_herdr_config "/Users/mm/.config/herdr/herdr.sock"
    for socket in /Users/mm/.config/herdr/sessions/*/herdr.sock; do
      reload_herdr_config "$socket"
    done
  '';
}

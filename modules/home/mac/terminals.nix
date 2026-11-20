{
  config,
  lib,
  pkgs,
  ...
}:
let
  rexDomain = "com.superlogical.rex";

  # Rex's imported themes, as it saved them after importing Ghostty's
  # TokyoNight Night. Rex stores the JSON as plist data, which toPlist cannot
  # write, so the plist is built here with a data element.
  rexThemesFile = ../../../config/rex/custom-themes.json;
  rexThemes = lib.importJSON rexThemesFile;
  rexThemeId = name: (lib.findFirst (theme: theme.name == name) null rexThemes).id;
  rexThemesPlist = pkgs.runCommand "rex-custom-themes.plist" { nativeBuildInputs = [ pkgs.jq ]; } ''
    themes=$(jq -cj . ${rexThemesFile} | base64 | tr -d '\n')
    cat > $out <<EOF
    <?xml version="1.0" encoding="UTF-8"?>
    <!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
    <plist version="1.0"><dict><key>Workspace.customThemes</key><data>$themes</data></dict></plist>
    EOF
  '';
in
{
  xdg.configFile = {
    "ghostty".source = config.lib.nixConfig.file "config/ghostty";
    "rex/init.lua".source = config.lib.nixConfig.file "config/rex/init.lua";
  };

  home.file."Library/Application Support/com.mitchellh.ghostty/config.ghostty".source =
    config.lib.nixConfig.file "config/ghostty/config";

  # Rex is installed by hand and updates itself, so its CLI stays inside the
  # app bundle; this puts it on PATH where Rex itself expects it.
  home.file.".local/bin/rex".source =
    config.lib.file.mkOutOfStoreSymlink "/Applications/Rex Beta.app/Contents/Helpers/Rex Server.rex/Contents/MacOS/rex";

  # Rex's Settings window saves to its defaults domain; a switch writes these
  # keys back over any change made there. Window frames, onboarding and
  # migration flags, the sidebar's collapsed state, session and activity
  # records and the cached server route are Rex's own state and stay out.
  targets.darwin.defaults.${rexDomain} = {
    "Workspace.darkThemeID" = rexThemeId "TokyoNight Night";
    "Workspace.lightThemeID" = "merino-light";
    "Workspace.interfaceStyle" = "useTheme";
    # Ghostty's font, size and horizontal padding. Rex has no separate italic
    # font (Ghostty's italics are MonaspiceRn) and no per-feature control:
    # its ligature switch is left at its default.
    "Workspace.terminalFontName" = "MonaspiceNe Nerd Font Mono";
    "Workspace.terminalFontPointSize" = 13;
    "Workspace.terminalPaddingX" = 10;
    "Workspace.terminalFontSmoothing" = false;
    "Workspace.terminalUsesDisplayP3" = false;
    "Workspace.terminalCustomSchemeLinks" = "ask";
    "Workspace.appIconVisibility" = "menuBarOnly";
    "Workspace.tabStyle" = "vertical";
    "Workspace.tabIconTintMode" = "off";
    "Workspace.paneDensity" = "compact";
    "Workspace.paneHeaderVisibility" = "always";
    "Workspace.showsPaneHeaders" = true;
    "Workspace.inactivePaneFadeMode" = "both";
    "Workspace.usesStandardInactiveWindowPaneStyles" = true;
    "Workspace.sidebarPosition" = "right";
    "Workspace.sidebarShowsActivity" = false;
    "Workspace.activityGrouping" = "priority";
    "rex.agents.names-sessions" = false;
    "rex.server.automatic-restart-enabled" = true;
    "rex.server.block-plugin-reload-enabled" = false;
    "rex.server.run-mode" = "bundled";
    "rex.server.web-client-origin-patterns" = [ "https://rex.superlogical.com" ];
  };

  # `defaults import` merges top-level keys, so the domain's other keys stay.
  home.activation.rexCustomThemes = lib.hm.dag.entryAfter [ "writeBoundary" ] ''
    run /usr/bin/defaults import ${rexDomain} ${rexThemesPlist}
  '';

  home.sessionVariables = {
    TERMINFO_DIRS = "/usr/share/terminfo";

    TERMINAL = "/Applications/Ghostty.app/Contents/MacOS/ghostty";
  };
}

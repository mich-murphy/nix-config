{...}: {
  programs.yazi = {
    enable = true;
    enableFishIntegration = true;
    shellWrapperName = "y";
    settings = {
      log = {
        enabled = false;
      };
      manager = {
        show_hidden = true;
        sort_by = "mtime";
        sort_dir_first = true;
      };
    };
    theme = {
      status = {
        separator = {
          opening = "";
          closing = "";
        };
      };
    };
  };
}

{...}: {
  programs.yazi = {
    enable = true;
    shellWrapperName = "y";
    settings = {
      mgr = {
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

{
  lib,
  config,
  pkgs,
  ...
}:
with lib; let
  cfg = config.common.git;
in {
  options.common.git = {
    enable = mkEnableOption "Enable Git with personalised settings";
  };

  config = mkIf cfg.enable {
    programs.git = {
      enable = true;
      userName = "mich-murphy";
      userEmail = "github@elmurphy.com";
      ignores = [
        # general
        ".DS_Store"
        ".AppleDouble"
        "LSOverride"
        # icon must end with two \r
        "Icon^M^M"
        # thumbnails
        "._*"
        # files which may appear on root of volume
        ".DocumentRevisions-V100"
        ".fseventsd"
        ".Spotlight-V100"
        ".TemporaryItems"
        ".Trashes"
        ".VolumeIcon.icns"
        ".com.apple.timemachine.donotpresent"
        # directories potentially created on remote AFP share
        ".AppleDB"
        ".AppleDesktop"
        "Network Trash Folder"
        "Temporary Items"
        ".apdisk"
        # python
        ".venv"
        ".envrc"
        ".tox"
        "*.pyc"
      ];
      extraConfig = {
        init.defaultBranch = "main";
        fetch.prune = true;
        pull.rebase = true;
        merge.conflictStyle = "zdiff3";
        merge.autoStash = true;
        rebase.autoStash = true;
        push.autoSetupRemote = true;
        commit.verbose = 1;
        commit.cleanup = "scissors";
        stash.showPatch = true;
        branch.sort = "-committerdate";
        interactive.singleKey = true;
        rerere.enabled = true;
        help.autoCorrect = true;
        color = {
          "branch" = {
            current = "yellow reverse";
            local = "yellow";
            remote = "green";
          };
          "status" = {
            added = "yellow";
            changed = "green";
            untracked = "cyan";
          };
        };
        diff = {
          colorMoved = "default";
          algorithm = "histogram";
          interHunkContext = 3;
          "exiftool" = {
            textconv = "exiftool --composite -x 'Exiftool:*' -x 'File:*' -g0";
            cachetextconv = true;
            xfuncname = "^-.*$";
          };
          "pandoc-to-markdown" = {
            textconv = "pandoc --to markdown";
            cachetextconv = true;
          };
        };
      };
      delta = {
        enable = true;
        options = {
          hyperlinks = true;
          line-numbers = true;
          navigate = true;
          side-by-side = true;
          syntax-theme = "tokyonight";
          minus-style = "syntax '#37222c'";
          minus-non-emph-style = "syntax '#37222c'";
          minus-emph-style = "syntax '#713137'";
          minus-empty-line-marker-style = "syntax '#37222c'";
          line-numbers-minus-style = "#b2555b";
          plus-style = "syntax '#20303b'";
          plus-non-emph-style = "syntax '#20303b'";
          plus-emph-style = "syntax '#2c5a66'";
          plus-empty-line-marker-style = "syntax '#20303b'";
          line-numbers-plus-style = "#266d6a";
          line-numbers-zero-style = "#3b4261";
        };
      };
      # specify diff application for file types
      attributes = [
        "*.bash diff=bash"
        "*.c diff=cpp"
        "*.cpp diff=cpp"
        "*.cs diff=csharp"
        "*.css diff=css"
        "*.ex diff=elixir"
        "*.exs diff=elixir"
        "*.go diff=golang"
        "*.h diff=c"
        "*.htm diff=html"
        "*.html diff=html"
        "*.java diff=java"
        "*.kt diff=kotlin"
        "*.kts diff=kotlin"
        "*.ktm diff=kotlin"
        "*.md diff=markdown"
        "*.m diff=matlab"
        "*.pas diff=pascal"
        "*.inc diff=pascal"
        "*.pp diff=pascal"
        "*.pl diff=perl"
        "*.php diff=php"
        "*.py diff=python"
        "*.pyi diff=python"
        "*.rb diff=ruby"
        "*.rs diff=rust"
        "*.sass diff=css"
        "*.scss diff=css"
        "*.sh diff=bash"
        "*.tex diff=tex"
        "*.zsh diff=bash"
        "*.avif diff=exiftool"
        "*.bmp diff=exiftool"
        "*.gif diff=exiftool"
        "*.jpeg diff=exiftool"
        "*.jpg diff=exiftool"
        "*.png diff=exiftool"
        "*.webp diff=exiftool"
        "*.docx diff=pandoc-to-markdown"
        "*.odt diff=pandoc-to-markdown"
        "*.ipynb diff=pandoc-to-markdown"
        "*.rtf diff=pandoc-to-markdown"
      ];
      aliases = {
        switch-recent = "!git branch --sort=-committerdate --format='%(refname:short)' | fzf --preview='git log --date=relative --color main..{}' | xargs git switch";
        rm-merged = "!git branch --format '%(refname:short) %(upstream:track)' | awk '$2 == \"[gone]\" { print $1 }' | xargs -r git branch -D";
        sync = "!git switch main && git pull && git rm-merged";
        edit-unmerged = "!git diff --name-only --diff-filter U | xargs -r $(git var GIT_EDITOR)";
      };
    };

    home.packages = with pkgs; [
      delta
      exiftool
      pandoc
      gh
    ];
  };
}

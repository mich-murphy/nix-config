{ config, ... }:
let
  finderCustomFolderIcon = "Icon[\r]";
  key = name: "${config.home.homeDirectory}/.ssh/${name}.pub";
  workConnectionTag = "github-bc";
  githubHosts = "github.com,ssh.github.com";
in
{
  programs.ssh.settings = {
    "Match host ${githubHosts} tagged ${workConnectionTag}" = {
      User = "git";
      IdentityFile = key "github_bc";
    };
    "Match host ${githubHosts} !tagged ${workConnectionTag}" = {
      User = "git";
      IdentityFile = key "github";
    };
  };

  programs.git = {
    includes = [
      {
        condition = "gitdir:~/businesscraft/";
        contents = {
          user = {
            name = "michaelmbc";
            email = "michaelmbc@users.noreply.github.com";
          };
          core.sshCommand = "ssh -P ${workConnectionTag}";
        };
      }
    ];
    ignores = [
      ".DS_Store"
      ".AppleDouble"
      "LSOverride"
      finderCustomFolderIcon
      "._*"
      ".DocumentRevisions-V100"
      ".fseventsd"
      ".Spotlight-V100"
      ".TemporaryItems"
      ".Trashes"
      ".VolumeIcon.icns"
      ".com.apple.timemachine.donotpresent"
      ".AppleDB"
      ".AppleDesktop"
      "Network Trash Folder"
      "Temporary Items"
      ".apdisk"
    ];
    settings.user = {
      name = "mich-murphy";
      email = "github@elmurphy.com";
    };
  };
}

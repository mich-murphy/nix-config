let
  finderCustomFolderIcon = "Icon[\r]";
in
{
  programs.git = {
    includes = [
      {
        condition = "gitdir:~/businesscraft/";
        contents = {
          user = {
            name = "michaelmbc";
            email = "michaelmbc@users.noreply.github.com";
          };
          core.sshCommand = "ssh -P github-bc";
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

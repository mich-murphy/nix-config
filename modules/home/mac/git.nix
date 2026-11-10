# Git on the Macs: macOS ignores, the personal identity, and the work identity under ~/businesscraft/.
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
          core.sshCommand = "ssh -i ~/.ssh/github_bc";
        };
      }
    ];
    ignores = [
      ".DS_Store"
      ".AppleDouble"
      "LSOverride"
      # Finder custom-folder icon files are named "Icon\r".
      "Icon[\r]"
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
    ];
    settings.user = {
      name = "mich-murphy";
      email = "github@elmurphy.com";
    };
  };
}

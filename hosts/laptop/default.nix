{
  imports = [ ../../modules/darwin ];

  networking = {
    computerName = "laptop";
    hostName = "laptop";
  };

  nixpkgs.hostPlatform = "aarch64-darwin";

  homebrew.brews = [ "llama.cpp" ];

  system.primaryUser = "michael";

  system.stateVersion = 7;
}

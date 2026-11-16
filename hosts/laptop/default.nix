{
  imports = [ ../../modules/darwin ];

  networking = {
    computerName = "laptop";
    hostName = "laptop";
  };

  nixpkgs.hostPlatform = "aarch64-darwin";

  system.primaryUser = "michael";

  system.stateVersion = 7;
}

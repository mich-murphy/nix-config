{
  imports = [ ../../modules/darwin ];

  networking = {
    computerName = "macbook";
    hostName = "macbook";
  };

  nixpkgs.hostPlatform = "aarch64-darwin";

  system.primaryUser = "mm";

  # Set once at first activation; never bump it to track releases.
  system.stateVersion = 4;
}

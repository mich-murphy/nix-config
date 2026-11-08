# M5 MacBook Pro.
{
  imports = [ ../../modules/darwin ];

  networking = {
    computerName = "laptop";
    hostName = "laptop";
  };

  nixpkgs.hostPlatform = "aarch64-darwin";

  # Set once at first activation; never bump it to track releases.
  system.stateVersion = 7;
}

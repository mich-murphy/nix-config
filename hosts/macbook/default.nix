# Current M2 MacBook Air. Replaced by `laptop` once the M5 has taken over.
{ ... }:
{
  imports = [ ../../modules/darwin ];

  networking = {
    computerName = "macbook";
    hostName = "macbook";
  };

  nixpkgs.hostPlatform = "aarch64-darwin";

  # Set once at first activation; never bump it to track releases.
  system.stateVersion = 4;
}

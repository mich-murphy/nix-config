let
  lidAppleLogoOff = 0;
in
{
  imports = [ ../../modules/darwin ];

  networking = {
    computerName = "laptop";
    hostName = "laptop";
  };

  nixpkgs.hostPlatform = "aarch64-darwin";

  system.primaryUser = "michael";

  system.defaults.CustomSystemPreferences."com.apple.CoreBrightness".LEBacklightMode =
    lidAppleLogoOff;

  system.stateVersion = 7;
}

{
  imports = [ ../../modules/darwin ];

  networking = {
    computerName = "laptop";
    hostName = "laptop";
  };

  nixpkgs.hostPlatform = "aarch64-darwin";

  system.primaryUser = "michael";

  # Turn off the illuminated Apple logo on the lid (PROLED XDR MacBook Pros).
  # Apple documents this as `sudo defaults write com.apple.CoreBrightness ...`,
  # which writes root's domain; this option runs the same command as root, so
  # the bare domain matches it. Removing this does not undo it; run
  # `sudo defaults delete com.apple.CoreBrightness LEBacklightMode`.
  system.defaults.CustomSystemPreferences."com.apple.CoreBrightness".LEBacklightMode = 0;

  system.stateVersion = 7;
}

{
  config,
  modulesPath,
  ...
}: {
  # nixos system configuration

  imports = [
    (modulesPath + "/installer/scan/not-detected.nix")
    (modulesPath + "/profiles/qemu-guest.nix")
    ./disks.nix # import disko configuration
  ];

  # specify boot device and enable efi boot
  boot.loader.grub = {
    devices = ["/dev/sda"];
    efiSupport = true;
    efiInstallAsRemovable = true;
  };

  # Enable OpenGL
  hardware.opengl = {
    enable = true;
    driSupport = true;
    driSupport32Bit = true;
  };

  # Load nvidia driver for Xorg and Wayland
  services.xserver.videoDrivers = ["nvidia"];

  hardware.nvidia = {
    modesetting.enable = true; # modesetting is required.
    powerManagement.enable = false; # resolve issues during wake from sleep
    powerManagement.finegrained = false; # turn off gpu when not in use
    open = false; # use alpha quality open source kernel module

    # you may need to select the appropriate driver version for your specific GPU.
    package = config.boot.kernelPackages.nvidiaPackages.production;
  };
}

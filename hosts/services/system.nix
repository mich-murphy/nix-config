{modulesPath, ...}: {
  # nixos system configuration

  imports = [
    (modulesPath + "/installer/scan/not-detected.nix")
    (modulesPath + "/profiles/qemu-guest.nix")
    ./disks.nix # import disko configuration
  ];

  # specify boot device and enable efi boot
  boot = {
    growPartition = true; # enable growing of root partition on boot
    loader.grub = {
      devices = ["/dev/sda"];
      efiSupport = true;
      efiInstallAsRemovable = true;
    };
  };
}

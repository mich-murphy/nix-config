{modulesPath, ...}: {
  # nixos system configuration

  imports = [
    (modulesPath + "/installer/scan/not-detected.nix")
    (modulesPath + "/virtualisation/proxmox-lxc.nix")
    (modulesPath + "/profiles/qemu-guest.nix")
  ];

  # specify boot device and enable efi boot
  boot = {
    isContainer = true;
    growPartition = true; # enable growing of root partition on boot
    loader.grub = {
      devices = ["nodev"];
      efiSupport = true;
      efiInstallAsRemovable = true;
    };
  };

  fileSystems."/" = {device = "/dev/disk/by-label/nixos";};
}

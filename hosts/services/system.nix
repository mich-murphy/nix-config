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

  # mount shared samba drive as deluge user and group
  fileSystems."/mnt/torrents" = {
    device = "//10.77.2.102/data/downloads/torrents";
    fsType = "cifs";
    options = let
      # this line prevents hanging on network split
      automount_opts = "x-systemd.automount,noauto,x-systemd.idle-timeout=60,x-systemd.device-timeout=5s,x-systemd.mount-timeout=5s,dir_mode=0775,file_mode=0775";
    in ["${automount_opts},credentials=${config.age.secrets.sambaPass.path},uid=${toString config.users.users.deluge.uid},gid=${toString config.users.groups.deluge.gid}"];
  };

  # agenix managed samba secret
  age.secrets.sambaPass.file = ../../secrets/sambaPass.age;
}

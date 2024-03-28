{
  config,
  inputs,
  ...
}: {
  # nixos system configuration

  imports = [
    ./disks.nix
    inputs.impermanence.nixosModules.impermanence
  ];

  boot.loader.grub = {
    enable = true;
    device = "/dev/sda"; # specify boot device
  };

  environment = {
    # set directories to be persisted after reboot
    persistence."/nix/persist" = {
      directories = [
        "/etc/nixos"
        "/var/log"
        "/var/lib"
        "/root/.ssh"
      ];
    };
    # set persistance for host ssh keys
    etc = {
      "machine-id".source = "/nix/persist/etc/machine-id";
      "ssh/ssh_host_rsa_key".source = "/nix/persist/etc/ssh/ssh_host_rsa_key";
      "ssh/ssh_host_rsa_key.pub".source = "/nix/persist/etc/ssh/ssh_host_rsa_key.pub";
      "ssh/ssh_host_ed25519_key".source = "/nix/persist/etc/ssh/ssh_host_ed25519_key";
      "ssh/ssh_host_ed25519_key.pub".source = "/nix/persist/etc/ssh/ssh_host_ed25519_key.pub";
    };
  };

  # mount shared samba drive as media group
  fileSystems."/mnt/data" = {
    device = "//10.77.2.102/data";
    fsType = "cifs";
    options = let
      # this line prevents hanging on network split
      automount_opts = "x-systemd.automount,noauto,x-systemd.idle-timeout=60,x-systemd.device-timeout=5s,x-systemd.mount-timeout=5s,dir_mode=0775,file_mode=0775";
    in ["${automount_opts},credentials=${config.age.secrets.sambaPass.path},gid=${toString config.users.groups.media.gid}"];
  };

  # agenix managed samba secret
  age.secrets.sambaPass.file = ../../secrets/sambaPass.age;
}

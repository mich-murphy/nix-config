{...}: {
  disko.devices = {
    disk.main = {
      device = "/dev/sda";
      type = "disk";
      content = {
        type = "gpt";
        partitions = {
          boot = {
            size = "1M";
            type = "EF02"; # for grub MBR
          };
          ESP = {
            name = "ESP";
            size = "512M";
            type = "EF00";
            content = {
              type = "filesystem";
              format = "vfat";
              mountpoint = "/boot";
            };
          };
          swap = {
            size = "16G";
            content = {
              type = "swap";
              randomEncryption = true;
              resumeDevice = true; # resume from hiberation from this device
            };
          };
          root = {
            size = "100%";
            content = {
              type = "filesystem";
              format = "ext4";
              mountpoint = "/";
            };
          };
        };
      };
    };
    # nodev."/" = {
    #   fsType = "tmpfs";
    #   mountOptions = [
    #     "size=6G"
    #     "defaults"
    #     "mode=755"
    #   ];
    # };
  };
}

{pkgs, ...}: {
  # system applications

  imports = [
    ../../nixos/modules
  ];

  common = {
    tailscale.enable = false;
  };

  environment = {
    systemPackages = [
      pkgs.vim
    ];
  };

  services = {
    qemuGuest.enable = true; # used for hypervisor operations
    ollama = {
      enable = true; # used for running various ai llms
      home = "/data/ollama";
      models = "/data/ollama/models";
      acceleration = "cuda";
    };
  };
}

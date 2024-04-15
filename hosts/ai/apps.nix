{pkgs, ...}: {
  # system applications

  imports = [
    ../../nixos/modules
  ];

  common.tailscale.enable = true;

  environment = {
    systemPackages = [
      pkgs.vim
      pkgs.git
      pkgs.nvtopPackages.nvidia # gpu utilisation metrics
      pkgs.glib # invokeai dependency
    ];
  };

  services = {
    qemuGuest.enable = true; # used for hypervisor operations
    ollama = {
      enable = true; # used for running llms
      acceleration = "cuda";
    };
    # invokeai.enable = true; # deploy .#ai -- --impure to handle broken python injector
  };

  virtualisation = {
    docker = {
      enable = true; # use docker for virtualisation
      autoPrune = {
        enable = true;
        dates = "weekly"; # prune docker resources weekly
      };
    };
    oci-containers = {
      backend = "docker";
      # web ui for ollama
      containers."open-webui" = {
        autoStart = true;
        image = "ghcr.io/open-webui/open-webui:main";
        environment = {
          TZ = "Australia/Melbourne";
          OLLAMA_API_BASE_URL = "http://127.0.0.1:11434/api";
          OLLAMA_BASE_URL = "http://127.0.0.1:11434";
        };
        ports = ["3000:8080"];
        volumes = [
          "/var/lib/open-webui:/app/backend/data"
        ];
        extraOptions = [
          "--network=host"
          "--add-host=host.containers.internal:host-gateway"
        ];
      };
    };
  };
}

{
  pkgs,
  lib,
  config,
  ...
}: let
  cfg = config.common.smokeping;
in {
  options.common.smokeping = {
    enable = lib.mkEnableOption "Enable smokeping";
    domain = lib.mkOption {
      type = lib.types.str;
      default = "smokeping.pve.elmurphy.com";
      description = "Domain for smokeping";
    };
    hostAddress = lib.mkOption {
      type = lib.types.str;
      default = "127.0.0.1";
      description = "IP address of smokeping host";
    };
    port = lib.mkOption {
      type = lib.types.port;
      default = 8002;
      description = "Port for smokeping";
    };
    nginx = lib.mkOption {
      type = lib.types.bool;
      default = true;
      description = "Enable nginx reverse proxy with SSL";
    };
  };

  config = lib.mkIf cfg.enable {
    assertions = [
      {
        assertion = cfg.nginx -> config.services.nginx.enable == true;
        message = "Nginx needs to be enabled";
      }
    ];

    security.wrappers = {
      dig = {
        setuid = true;
        owner = "root";
        group = "root";
        source = "${pkgs.dig}/bin/dig";
      };
    };

    services = {
      smokeping = {
        enable = true;
        host = cfg.hostAddress;
        hostName = cfg.domain;
        imgUrl = "https://${cfg.domain}/cache";
        cgiUrl = "https://${cfg.domain}/smokeping.cgi";
        probeConfig = ''
          + FPing
          binary = ${config.security.wrapperDir}/fping

          + DNS
          binary = ${config.security.wrapperDir}/dig
        '';
        targetConfig = ''
          probe = FPing

          menu = Top
          title = Network Latency Grapher
          remark = Welcome to SmokePing

          + InternetSites
          menu = Internet Sites
          title = Internet Sites

          ++ YouTube
          menu = YouTube
          title = YouTube
          host = youtube.com

          ++ mich-murphy
          menu = mich-murphy
          title = mich-murphy
          host = mich-murphy.com

          ++ Google
          menu = Google
          title = google.com
          host = google.com

          + India
          menu = India
          title = India

          ++ Bangalore
          menu = Bangalore
          title = Indian Institute of Science
          host = iisc.ernet.in

          + DNS
          probe = DNS
          menu = DNS
          title = DNS Latency

          ++ GoogleDNS1
          menu = Google DNS 1
          title = Google DNS 8.8.8.8
          host = 8.8.8.8

          ++ GoogleDNS2
          menu = Google DNS 2
          title = Google DNS 8.8.4.4
          host = 8.8.4.4

          ++ CloudflareDNS1
          menu = Cloudflare DNS 1
          title = Cloudflare DNS 1.1.1.1
          host = 1.1.1.1

          ++ CloudflareDNS2
          menu = Cloudflare DNS 2
          title = Cloudflare DNS 1.0.0.1
          host = 1.0.0.1

          ++ Quad9DNS1
          menu = Quad9 DNS 1
          title = Quad9 DNS 9.9.9.9
          host = 9.9.9.9

          ++ Quad9DNS2
          menu = Quad9 DNS 2
          title = Quad9 DNS 149.112.112.112
          host = 149.112.112.112
        '';
      };
      nginx = lib.mkIf cfg.nginx {
        virtualHosts.smokeping.listen = [
          {
            addr = cfg.hostAddress;
            port = cfg.port;
          }
        ];
        virtualHosts."${cfg.domain}" = {
          forceSSL = true;
          useACMEHost = "elmurphy.com";
          locations."/" = {
            proxyPass = "http://${cfg.hostAddress}:${toString cfg.port}";
          };
        };
      };
    };
  };
}

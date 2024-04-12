{...}: {
  # security settings

  security = {
    sudo.execWheelOnly = true; # only wheel group can use sudo
    sudo.wheelNeedsPassword = false;
  };

  services = {
    # harden ssh configuration
    openssh = {
      enable = true;
      allowSFTP = false;
      settings = {
        PasswordAuthentication = false;
        KbdInteractiveAuthentication = false;
      };
      settings = {
        PermitRootLogin = "no";
      };
      extraConfig = ''
        AllowTcpForwarding yes
        X11Forwarding no
        AllowAgentForwarding no
        AllowStreamLocalForwarding no
        AuthenticationMethods publickey
      '';
      hostKeys = [
        {
          # default to ed25519 key generation
          path = "/etc/ssh/ssh_host_ed25519_key";
          type = "ed25519";
        }
      ];
    };
  };
}

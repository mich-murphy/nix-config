{config, ...}: {
  # user configuration

  networking.hostName = "services";
  time.timeZone = "Australia/Melbourne";

  users = {
    mutableUsers = false;
    users = {
      mm = {
        isNormalUser = true;
        home = "/home/mm";
        hashedPasswordFile = config.age.secrets.userPass.path;
        extraGroups = ["wheel" "deluge"];
        openssh.authorizedKeys.keys = [
          "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIEVyN0R5mTtfcbkmVXjicuvSRotJY4IOuetOgPyG2lg8"
        ];
      };
    };
  };

  # agenix managed user secrets
  age.secrets.userPass.file = ../../secrets/userPass.age;
}

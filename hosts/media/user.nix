{config, ...}: {
  # host and user configuration

  networking.hostName = "media";
  time.timeZone = "Australia/Melbourne";

  users = {
    groups.media = {};
    groups.media.gid = 985;
    mutableUsers = false;
    users = {
      mm = {
        isNormalUser = true;
        home = "/home/mm";
        hashedPasswordFile = config.age.secrets.userPass.path; # specify agenix file
        extraGroups = ["wheel" "media"];
        openssh.authorizedKeys.keys = [
          "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIEVyN0R5mTtfcbkmVXjicuvSRotJY4IOuetOgPyG2lg8"
        ];
      };
    };
  };

  # manage user password secret with agenix
  age.secrets.userPass.file = ../../secrets/userPass.age;
}

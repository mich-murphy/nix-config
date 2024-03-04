let
  media = "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIEVyN0R5mTtfcbkmVXjicuvSRotJY4IOuetOgPyG2lg8";
  services = "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIArOkDV7ftpd1Ul53eZ6V0hBoSS9D7rTMI8Kn9HlbCna";
  servers = [media services];
in {
  "userPass.age".publicKeys = servers;
  "nextcloudPass.age".publicKeys = servers;
  "acmeCredentials.age".publicKeys = servers;
  "objectStorage.age".publicKeys = servers;
  "mediaBorgPass.age".publicKeys = servers;
  "nextcloudBorgPass.age".publicKeys = servers;
  "freshrssPass.age".publicKeys = servers;
  "giteaDbPass.age".publicKeys = servers;
  "sambaPass.age".publicKeys = servers;
  "murmurPass.age".publicKeys = servers;
}

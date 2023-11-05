let
  media = "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIEVyN0R5mTtfcbkmVXjicuvSRotJY4IOuetOgPyG2lg8";
  storage = "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIArOkDV7ftpd1Ul53eZ6V0hBoSS9D7rTMI8Kn9HlbCna";
in {
  "userPass.age".publicKeys = [media storage];
  "nextcloudPass.age".publicKeys = [media];
  "acmeCredentials.age".publicKeys = [media];
  "objectStorage.age".publicKeys = [media];
  "mediaBorgPass.age".publicKeys = [media];
  "nextcloudBorgPass.age".publicKeys = [media];
  "freshrssPass.age".publicKeys = [media];
  "giteaDbPass.age".publicKeys = [media];
  "sambaPass.age".publicKeys = [media storage];
}

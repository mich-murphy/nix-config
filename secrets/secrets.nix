let
  media = "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIEVyN0R5mTtfcbkmVXjicuvSRotJY4IOuetOgPyG2lg8";
in {
  "userPass.age".publicKeys = [media];
  "nextcloudPass.age".publicKeys = [media];
  "acmeCredentials.age".publicKeys = [media];
  "objectStorage.age".publicKeys = [media];
  "mediaBorgPass.age".publicKeys = [media];
  "nextcloudBorgPass.age".publicKeys = [media];
  "freshrssPass.age".publicKeys = [media];
}

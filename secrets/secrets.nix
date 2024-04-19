let
  media = "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIEVyN0R5mTtfcbkmVXjicuvSRotJY4IOuetOgPyG2lg8";
  services = "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIOt6E5Phwiv2S2yed58vOBzsyeQJ/ZhiBDAA8j+gGyq7";
  servers = [media services];
in {
  "userPass.age".publicKeys = servers;
  "nextcloudPass.age".publicKeys = [media];
  "acmeCredentials.age".publicKeys = [media];
  "objectStorage.age".publicKeys = [media];
  "mediaBorgPass.age".publicKeys = [media];
  "nextcloudBorgPass.age".publicKeys = [media];
  "freshrssPass.age".publicKeys = [media];
  "giteaDbPass.age".publicKeys = [media];
  "sambaPass.age".publicKeys = servers;
  "murmurPass.age".publicKeys = [media];
  "delugePass.age".publicKeys = [services];
  "matrixSharedSecret.age".publicKeys = [media];
}

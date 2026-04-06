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
  "giteaDbPass.age".publicKeys = [media];
  "sambaPass.age".publicKeys = [media];
  "murmurPass.age".publicKeys = [media];
  "matrixSharedSecret.age".publicKeys = [media];
  "immichBorgPass.age".publicKeys = [media];
  "gitlabPass.age".publicKeys = [media];
  "gitlabDbPass.age".publicKeys = [media];
  "gitlabDbFile.age".publicKeys = [media];
  "gitlabJwsFile.age".publicKeys = [media];
  "gitlabOtpFile.age".publicKeys = [media];
  "gitlabSecretFile.age".publicKeys = [media];
  "netdataClaimToken.age".publicKeys = [media];
  "gitBorgPass.age".publicKeys = [media];
  "tailscaleAuthKey.age".publicKeys = [media];
  "paperlessPass.age".publicKeys = [media];
  "delugePass.age".publicKeys = [media];
}

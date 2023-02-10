let
  nix-media = "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIMne13aa88i97xAUqU33dk2FNz+w8OIMGi8LH4BCRFaN";
in
{
  "userPass.age".publicKeys = [ nix-media ];
  "nextcloudPass.age".publicKeys = [ nix-media ];
  "acmeCredentials.age".publicKeys = [ nix-media ];
  "objectStorage.age".publicKeys = [ nix-media ];
  "mediaBorgPass.age".publicKeys = [ nix-media ];
  "nextcloudBorgPass.age".publicKeys = [ nix-media ];
  "borgSSHKey.age".publicKeys = [ nix-media ];
}

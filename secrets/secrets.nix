let
  nix-media = "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIMne13aa88i97xAUqU33dk2FNz+w8OIMGi8LH4BCRFaN";
in
{
  "userPass.age".publicKeys = [ nix-media ];
  "nextcloudPass.age".publicKeys = [ nix-media ];
  "syncthingDevice.age".publicKeys = [ nix-media ];
  "syncthingPass.age".publicKeys = [ nix-media ];
  "objectStorage.age".publicKeys = [ nix-media ];
}

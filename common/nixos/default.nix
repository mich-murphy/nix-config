{ lib, config, pkgs, ... }:

{
  imports = [
    ./nextcloud.nix
    ./borgbackup.nix
  ];
}

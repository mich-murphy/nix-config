{ inputs, lib, ... }:
let
  extras = "${inputs.tokyonight}/extras";
in
{
  lib.tokyonight = {
    inherit extras;
    palette = (lib.importJSON "${extras}/opencode/tokyonight_night.json").defs;
  };
}

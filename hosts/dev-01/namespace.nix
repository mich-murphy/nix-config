{ pkgs, ... }:
let
  ns-buildx = pkgs.writeShellApplication {
    name = "ns-buildx";
    runtimeInputs = [ pkgs.namespace-cli ];
    text = ''
      token_file="''${NSC_TOKEN_FILE:-''${XDG_CONFIG_HOME:-$HOME/.config}/namespace/builder.json}"
      if [ ! -r "$token_file" ]; then
        echo "ns-buildx: no Namespace builder token at $token_file" >&2
        exit 1
      fi
      export NSC_TOKEN_FILE="$token_file"
      if [ "$#" -eq 0 ]; then
        set -- setup --background --name nsc-remote
      fi
      exec nsc docker buildx "$@"
    '';
  };
in
{
  home.packages = [ ns-buildx ];
}

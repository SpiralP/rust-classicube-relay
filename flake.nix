{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-25.11";
    flake-utils.url = "github:SpiralP/nix-flake-utils";
  };

  outputs = inputs@{ flake-utils, ... }:
    flake-utils.lib.makeOutputs inputs
      ({ lib, pkgs, makeRustPackage, dev, ... }: {
        default = makeRustPackage pkgs (self: {
          src = lib.sourceByRegex ./. [
            "^\.cargo(/.*)?$"
            "^build\.rs$"
            "^Cargo\.(lock|toml)$"
            "^README\.md$"
            "^src(/.*)?$"
          ];

          nativeBuildInputs = with pkgs; [
            pkg-config
            rustPlatform.bindgenHook
          ];

          dontUseCargoParallelTests = true;
        });
      });
}

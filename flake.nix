{
  description = "A lightweight Prometheus metrics exporter for Kopia backup repositories";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = {
    self,
    nixpkgs,
    flake-utils,
  }:
    flake-utils.lib.eachDefaultSystem (system: let
      pkgs = nixpkgs.legacyPackages.${system};
      kopia-exporter-pkg = pkgs.callPackage ./. {};
    in {
      packages = {
        default = kopia-exporter-pkg;
        kopia-exporter = kopia-exporter-pkg;
        fake-kopia = pkgs.runCommand "fake-kopia" {} ''
          mkdir -p $out/bin
          cp ${kopia-exporter-pkg}/bin/fake-kopia $out/bin/
        '';
      };

      checks = {
        vm-test = pkgs.callPackage ./nixos-vm-test.nix {
          kopia-exporter = self.packages.${system}.default;
        };

        alejandra-format =
          pkgs.runCommand "alejandra-format-check" {
            buildInputs = [pkgs.alejandra];
          } ''
            cd ${./.}
            alejandra --check .
            touch $out
          '';
      };

      devShells.default = pkgs.mkShell {
        buildInputs = with pkgs; [
          cargo
          rustc
          rustfmt
          clippy
          rust-analyzer
        ];
      };
    })
    // {
      nixosModules.default = import ./nixos-module.nix;
    };
}

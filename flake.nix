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

        docs = let
          markdownContent = import ./nixos-module/extract-options.nix {inherit pkgs;};
        in
          pkgs.runCommand "kopia-exporter-docs" {
            buildInputs = with pkgs; [pandoc];
          } ''
            # Write markdown content
            cat > kopia-exporter-options.md << 'EOF'
            ${markdownContent}
            EOF

            # Convert to HTML using pandoc and inject custom CSS
            pandoc -s -t html --metadata title="Kopia Exporter NixOS Module Options" \
              kopia-exporter-options.md -o kopia-exporter-options-temp.html

            # Inject custom CSS directly into the HTML
            sed 's|</head>|<style>/* Custom CSS for kopia-exporter documentation */body { max-width: 80em !important; }@media (max-width: 600px) { body { max-width: none !important; } }</style></head>|' \
              kopia-exporter-options-temp.html > kopia-exporter-options.html

            # Copy to output
            mkdir -p $out
            cp kopia-exporter-options.md $out/
            cp kopia-exporter-options.html $out/
          '';
      };

      checks = {
        vm-test = pkgs.callPackage ./nixos-module/nixos-vm-test.nix {
          kopia-exporter = self.packages.${system}.default;
        };

        vm-caching-test = pkgs.callPackage ./nixos-module/nixos-vm-caching-test.nix {
          kopia-exporter = self.packages.${system}.default;
        };

        vm-auth-userpass-test =
          (pkgs.callPackage ./nixos-module/nixos-vm-auth-test.nix {
            kopia-exporter = self.packages.${system}.default;
          }).userpass;

        vm-auth-credfile-test =
          (pkgs.callPackage ./nixos-module/nixos-vm-auth-test.nix {
            kopia-exporter = self.packages.${system}.default;
          }).credfile;

        alejandra-format =
          pkgs.runCommand "alejandra-format-check" {
            buildInputs = [pkgs.alejandra];
          } ''
            cd ${./.}
            alejandra --check .
            touch $out
          '';

        cargo-fmt =
          pkgs.runCommand "cargo-fmt-check" {
            buildInputs = [pkgs.cargo pkgs.rustfmt];
          } ''
            cd ${./.}
            cargo fmt --check
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
      nixosModules.default = import ./nixos-module/nixos-module.nix;
    };
}

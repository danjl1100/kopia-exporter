{
  lib,
  rustPlatform,
}:
rustPlatform.buildRustPackage rec {
  pname = "kopia-exporter";
  version = "0.1.0";

  src = lib.cleanSourceWith {
    src = ./.;
    filter = path: type: let
      baseName = baseNameOf path;
      relativePath = lib.removePrefix (toString ./. + "/") (toString path);
    in
      # Allow root Rust files and configuration
      baseName
      == "Cargo.toml"
      || baseName == "Cargo.lock"
      || baseName == "build.rs"
      || baseName == "LICENSE"
      || baseName == "README.md"
      || baseName == "default.nix"
      ||
      # Allow directories and contents for Rust source
      baseName == "src"
      || baseName == "tests"
      || lib.hasPrefix "src/" relativePath
      || lib.hasPrefix "tests/" relativePath;
  };

  cargoLock = {
    lockFile = ./Cargo.lock;
  };

  # Build both kopia-exporter and fake-kopia binaries
  cargoBuildFlags = ["--bin" "kopia-exporter" "--bin" "fake-kopia"];

  meta = with lib; {
    description = "A lightweight Prometheus metrics exporter for Kopia backup repositories";
    license = licenses.mit;
    maintainers = [];
    mainProgram = "kopia-exporter";
  };
}

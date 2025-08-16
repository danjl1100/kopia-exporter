{pkgs}: let
  lib = pkgs.lib;

  # Evaluate just the options part of our module
  module = import ./nixos-module.nix;

  # Create a minimal evaluation context
  mockConfig = {
    _module.args = {inherit pkgs;};
  };

  evaluated = module {
    config = mockConfig;
    inherit lib pkgs;
  };

  # Convert options to a simple format for documentation
  optionsToMd = options:
    lib.concatStringsSep "\n\n" (
      lib.mapAttrsToList (name: opt: ''
        ## services.kopia-exporter.${name}

        **Type:** ${opt.type.description or "unknown"}

        **Default:** ${
          if opt ? defaultText
          then opt.defaultText.text
          else if opt ? default
          then builtins.toJSON opt.default
          else "none"
        }

        **Description:** ${opt.description or "No description provided."}
      '')
      options
    );
in
  "# Kopia Exporter NixOS Module Options\n\n"
  + optionsToMd evaluated.options.services.kopia-exporter

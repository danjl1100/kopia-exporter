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
  optionsToMd = prefix: options:
    lib.concatStringsSep "\n\n" (
      lib.flatten (
        lib.mapAttrsToList (
          name: opt:
          # Skip internal _module options
            if name == "_module"
            then []
            else let
              fullName =
                if prefix != ""
                then "${prefix}.${name}"
                else name;
              currentSection = ''
                ## services.kopia-exporter.${fullName}

                **Type:** ${
                  if (opt ? type) && (opt.type ? description)
                  then opt.type.description
                  else "unknown"
                }

                **Default:** ${
                  if opt ? defaultText
                  then opt.defaultText.text
                  else if opt ? default
                  then builtins.toJSON opt.default
                  else "none"
                }

                **Description:** ${opt.description or "No description provided."}
              '';
              # Check if this is a submodule with nested options
              nestedSections =
                if (opt ? type) && (opt.type ? nestedTypes) && (opt.type.nestedTypes ? elemType) && (opt.type.nestedTypes.elemType ? getSubOptions)
                then optionsToMd fullName (opt.type.nestedTypes.elemType.getSubOptions {})
                else if (opt ? type) && (opt.type ? getSubOptions)
                then optionsToMd fullName (opt.type.getSubOptions {})
                else [];
            in
              [currentSection]
              ++ (
                if builtins.isList nestedSections
                then nestedSections
                else [nestedSections]
              )
        )
        options
      )
    );
in
  "# Kopia Exporter NixOS Module Options\n\n"
  + optionsToMd "" evaluated.options.services.kopia-exporter

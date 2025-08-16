#!/usr/bin/env bash

set -euo pipefail

# Use a simple nix-instantiate approach to extract options
cat > extract-options.nix << 'EOF'
let
  pkgs = import <nixpkgs> {};
  lib = pkgs.lib;
  
  # Evaluate just the options part of our module
  module = import ./nixos-module.nix;
  
  # Create a minimal evaluation context
  mockConfig = {
    _module.args = { inherit pkgs; };
  };
  
  evaluated = module {
    config = mockConfig;
    inherit lib pkgs;
  };
  
  # Convert options to a simple format for documentation
  optionsToMd = options: lib.concatStringsSep "\n\n" (
    lib.mapAttrsToList (name: opt: ''
      ## services.kopia-exporter.${name}
      
      **Type:** ${opt.type.description or "unknown"}
      
      **Default:** ${if opt ? defaultText then opt.defaultText.text else if opt ? default then builtins.toJSON opt.default else "none"}
      
      **Description:** ${opt.description or "No description provided."}
    '') options
  );
  
in
  "# Kopia Exporter NixOS Module Options\n\n" + 
  optionsToMd evaluated.options.services.kopia-exporter
EOF

# Generate the markdown documentation
nix-instantiate --eval --strict extract-options.nix | sed 's/^"//; s/"$//' | sed 's/\\n/\n/g' > kopia-exporter-options.md

# Convert to HTML using pandoc (if available)
if command -v pandoc >/dev/null 2>&1; then
  pandoc -s -t html --metadata title="Kopia Exporter NixOS Module Options" \
    kopia-exporter-options.md -o kopia-exporter-options.html
  echo "Generated documentation:"
  echo "  - kopia-exporter-options.md (Markdown)"
  echo "  - kopia-exporter-options.html (HTML)"
else
  echo "Generated documentation:"
  echo "  - kopia-exporter-options.md (Markdown)"
  echo "  - pandoc not found, HTML generation skipped"
fi

# Clean up
rm extract-options.nix
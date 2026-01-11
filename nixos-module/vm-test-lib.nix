# Shared utilities for NixOS VM tests
{
  pkgs,
  kopia-exporter,
}: {
  # Base VM configuration shared across all tests
  baseVmConfig = {
    imports = [./nixos-module.nix];

    # Basic system configuration
    boot.loader.grub.enable = false;

    # Add curl for testing HTTP endpoints
    environment.systemPackages = with pkgs; [curl];

    # Minimal system requirements
    documentation.enable = false;
    documentation.nixos.enable = false;
    system.stateVersion = "23.11";
  };

  # Create a basic kopia-exporter service configuration
  mkServiceConfig = {
    bind ? "0.0.0.0:9090",
    extraConfig ? {},
  }:
    {
      enable = true;
      inherit bind;
      kopiaBin = "${kopia-exporter}/bin/fake-kopia";
    }
    // extraConfig;

  # Common test script boilerplate
  mkTestScript = {
    imports ? ["time"],
    customTests ? "",
    testDescription ? "VM test",
  }: ''
    ${builtins.concatStringsSep "\n" (builtins.map (imp: "import ${imp}") imports)}

    # Start the VM
    machine.start()
    machine.wait_for_unit("multi-user.target")

    # Wait for the kopia-exporter service to start
    machine.wait_for_unit("kopia-exporter.service")

    # Give the service a moment to bind to the port
    time.sleep(2)

    ${customTests}

    print("${testDescription} completed!")
  '';
}

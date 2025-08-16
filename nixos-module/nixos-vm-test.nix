{
  pkgs ? import <nixpkgs> {},
  system ? builtins.currentSystem,
  kopia-exporter ? pkgs.callPackage ../. {},
}:
pkgs.nixosTest {
  name = "kopia-exporter-service";

  nodes.machine = {
    config,
    lib,
    pkgs,
    ...
  }: {
    imports = [./nixos-module.nix];

    # Test the module's default package resolution (this should work with proper ../. references)
    # Comment out the override to test the module's own package resolution
    # services.kopia-exporter.package = kopia-exporter;

    # Basic system configuration
    boot.loader.grub.enable = false;
    networking.hostName = "kopia-exporter-test";

    # Enable the kopia-exporter service
    services.kopia-exporter = {
      enable = true;
      bind = "0.0.0.0:9090";
      kopiaBin = "${kopia-exporter}/bin/fake-kopia";
    };

    # Add curl for testing HTTP endpoints
    environment.systemPackages = with pkgs; [curl];

    # Minimal system requirements
    documentation.enable = false;
    documentation.nixos.enable = false;
    system.stateVersion = "23.11";
  };

  testScript = ''
    import time

    # Start the VM
    machine.start()
    machine.wait_for_unit("multi-user.target")

    # Wait for the kopia-exporter service to start
    machine.wait_for_unit("kopia-exporter.service")

    # Give the service a moment to bind to the port
    time.sleep(2)

    # Test the root endpoint
    machine.succeed("curl -f http://localhost:9090/")

    # Test the metrics endpoint
    metrics_output = machine.succeed("curl -f http://localhost:9090/metrics")

    # Verify the metrics contain expected Prometheus format
    assert "kopia_snapshots_by_retention" in metrics_output
    assert "kopia_snapshot_total_size_bytes" in metrics_output

    # Test that invalid endpoints return 404
    machine.fail("curl -f http://localhost:9090/invalid")

    print("All tests passed!")
  '';
}

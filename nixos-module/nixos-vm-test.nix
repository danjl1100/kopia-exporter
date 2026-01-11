{
  pkgs ? import <nixpkgs> {},
  system ? builtins.currentSystem,
  kopia-exporter ? pkgs.callPackage ../. {},
}: let
  vmTestLib = import ./vm-test-lib.nix {inherit pkgs kopia-exporter;};

  # Test basic endpoints (root and metrics)
  testBasicEndpoints = {port ? "9090"}: ''
    # Test the root endpoint
    machine.succeed("curl -f http://localhost:${port}/")

    # Test the metrics endpoint
    metrics_output = machine.succeed("curl -f http://localhost:${port}/metrics")

    # Verify the metrics contain expected Prometheus format
    assert "kopia_snapshots_by_retention" in metrics_output
    assert "kopia_snapshot_total_size_bytes" in metrics_output

    # Test that invalid endpoints return 404
    machine.fail("curl -f http://localhost:${port}/invalid")
  '';
in
  pkgs.testers.nixosTest {
    name = "kopia-exporter-service";

    nodes.machine = {
      config,
      lib,
      pkgs,
      ...
    }:
      vmTestLib.baseVmConfig
      // {
        networking.hostName = "kopia-exporter-test";

        # Enable the kopia-exporter service with defaults
        services.kopia-exporter = vmTestLib.mkServiceConfig {};

        # Test the module's default package resolution (this should work with proper ../. references)
        # Comment out the override to test the module's own package resolution
        # services.kopia-exporter.package = kopia-exporter;
      };

    testScript = vmTestLib.mkTestScript {
      customTests = testBasicEndpoints {};
      testDescription = "Basic service test";
    };
  }

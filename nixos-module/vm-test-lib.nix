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

  # Common HTTP endpoint tests
  httpTests = {
    # Test basic endpoints (root and metrics)
    basicEndpoints = {port ? "9090"}: ''
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

    # Test authentication (requires auth config in service)
    authentication = {
      user,
      password,
      port ? "9090",
      description ? "authentication test",
    }: ''
      print("Testing ${description}")

      # Test unauthenticated request - should fail with 401
      machine.fail("curl -f http://localhost:${port}/metrics")

      # Verify 401 response with proper headers
      auth_response = machine.succeed("curl -s -w '%{http_code}' http://localhost:${port}/metrics")
      assert "401" in auth_response

      # Test with correct credentials
      correct_auth = base64.b64encode(b"${user}:${password}").decode('ascii')
      metrics_output = machine.succeed(f"curl -f -H 'Authorization: Basic {correct_auth}' http://localhost:${port}/metrics")

      # Verify the metrics contain expected Prometheus format
      assert "kopia_snapshots_by_retention" in metrics_output
      assert "kopia_snapshot_total_size_bytes" in metrics_output

      # Test with incorrect credentials
      wrong_auth = base64.b64encode(b"wrong:wrong").decode('ascii')
      machine.fail(f"curl -f -H 'Authorization: Basic {wrong_auth}' http://localhost:${port}/metrics")

      # Test that root endpoint also requires auth
      machine.fail("curl -f http://localhost:${port}/")
      machine.succeed(f"curl -f -H 'Authorization: Basic {correct_auth}' http://localhost:${port}/")
    '';

    # Test caching behavior
    caching = {
      logPath,
      port ? "9090",
      cacheSeconds ? 1,
      description ? "caching test",
    }: ''
      print("Testing ${description}")

      # Clear any existing log
      machine.succeed("rm -f ${logPath}")

      # Make 3 rapid requests within cache window
      machine.succeed("curl -f http://localhost:${port}/metrics")
      machine.succeed("curl -f http://localhost:${port}/metrics")
      machine.succeed("curl -f http://localhost:${port}/metrics")

      # Check that fake-kopia was called only once due to caching
      log_content = machine.succeed("cat ${logPath} || echo 'no log'")
      call_count = len([line for line in log_content.strip().split('\n') if line.strip() == 'invocation'])
      assert call_count == 1, f"Expected 1 kopia call due to caching, got {call_count}. Log: {log_content}"

      # Wait for cache to expire and make another request
      time.sleep(${toString (cacheSeconds + 1)})
      machine.succeed("curl -f http://localhost:${port}/metrics")

      # Should now have 2 calls total
      log_content = machine.succeed("cat ${logPath}")
      call_count = len([line for line in log_content.strip().split('\n') if line.strip() == 'invocation'])
      assert call_count == 2, f"Expected 2 kopia calls after cache expiry, got {call_count}. Log: {log_content}"
    '';
  };
}

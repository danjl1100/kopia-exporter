{
  pkgs ? import <nixpkgs> {},
  system ? builtins.currentSystem,
  kopia-exporter ? pkgs.callPackage ../. {},
}:
pkgs.nixosTest {
  name = "kopia-exporter-caching";

  nodes.machine = {
    config,
    lib,
    pkgs,
    ...
  }: {
    imports = [./nixos-module.nix];

    # Basic system configuration
    boot.loader.grub.enable = false;
    networking.hostName = "kopia-exporter-caching-test";

    # Enable the kopia-exporter service with caching test configuration
    services.kopia-exporter = {
      enable = true;
      bind = "0.0.0.0:9090";
      kopiaBin = "${kopia-exporter}/bin/fake-kopia";
      cacheSeconds = 1; # Short cache for faster testing
      environment = {
        FAKE_KOPIA_LOG = "/home/kopia-exporter/fake-kopia-test.log";
      };
      # Override user home for testing
      user = "kopia-exporter";
    };

    # Configure user with custom home for easier testing
    users.users.kopia-exporter = {
      home = lib.mkForce "/home/kopia-exporter";
      createHome = lib.mkForce true;
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

    # Test caching behavior by making multiple requests and checking fake-kopia log
    log_path = "/home/kopia-exporter/fake-kopia-test.log"

    # Clear any existing log
    machine.succeed(f"rm -f {log_path}")

    # Make 3 rapid requests within cache window (1 second)
    machine.succeed("curl -f http://localhost:9090/metrics")
    machine.succeed("curl -f http://localhost:9090/metrics")
    machine.succeed("curl -f http://localhost:9090/metrics")

    # Check that fake-kopia was called only once due to caching
    log_content = machine.succeed(f"cat {log_path} || echo 'no log'")
    call_count = len([line for line in log_content.strip().split('\n') if line.strip() == 'invocation'])
    assert call_count == 1, f"Expected 1 kopia call due to caching, got {call_count}. Log: {log_content}"

    # Wait for cache to expire and make another request
    time.sleep(2)  # Cache expires after 1 second
    machine.succeed("curl -f http://localhost:9090/metrics")

    # Should now have 2 calls total
    log_content = machine.succeed(f"cat {log_path}")
    call_count = len([line for line in log_content.strip().split('\n') if line.strip() == 'invocation'])
    assert call_count == 2, f"Expected 2 kopia calls after cache expiry, got {call_count}. Log: {log_content}"

    print("Caching behavior test passed!")
  '';
}

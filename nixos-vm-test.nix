{
  pkgs ? import <nixpkgs> {},
  system ? builtins.currentSystem,
  kopia-exporter ? pkgs.callPackage ./. {},
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

    # Override the package option directly
    services.kopia-exporter.package = kopia-exporter;

    # Basic system configuration
    boot.loader.grub.enable = false;
    networking.hostName = "kopia-exporter-test";

    # Enable the kopia-exporter service
    services.kopia-exporter = {
      enable = true;
      bind = "0.0.0.0:9090";
      kopiaBin = "${pkgs.writeShellScript "fake-kopia" ''
        #!/bin/sh
        # Fake kopia binary that returns sample JSON data
        if [ "$1" = "snapshot" ] && [ "$2" = "list" ] && [ "$3" = "--json" ]; then
          cat ${pkgs.writeText "sample-kopia-output.json" ''
          [
            {
              "id": "test-snapshot-1",
              "source": {"host": "test-host", "userName": "test-user", "path": "/test-path"},
              "description": "",
              "startTime": "2025-08-15T12:00:00Z",
              "endTime": "2025-08-15T12:01:00Z",
              "stats": {
                "totalSize": 1000000,
                "excludedTotalSize": 0,
                "fileCount": 100,
                "cachedFiles": 50,
                "nonCachedFiles": 50,
                "dirCount": 10,
                "excludedFileCount": 0,
                "excludedDirCount": 0,
                "ignoredErrorCount": 0,
                "errorCount": 0
              },
              "rootEntry": {
                "name": "test-path",
                "type": "d",
                "mode": "0755",
                "mtime": "2025-08-15T12:00:00Z",
                "obj": "test-obj-id",
                "summ": {
                  "size": 1000000,
                  "files": 100,
                  "symlinks": 0,
                  "dirs": 10,
                  "maxTime": "2025-08-15T12:00:00Z",
                  "numFailed": 0
                }
              },
              "retentionReason": ["latest-1", "daily-1"]
            }
          ]
        ''}
        else
          echo "Unknown kopia command: $*" >&2
          exit 1
        fi
      ''}";
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

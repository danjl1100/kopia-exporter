{
  pkgs ? import <nixpkgs> {},
  system ? builtins.currentSystem,
  kopia-exporter ? pkgs.callPackage ../. {},
}: let
  # Common VM configuration
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

  # Common test functions
  commonAuthTestScript = {
    expectedUser,
    expectedPass,
    testDescription,
  }: ''
    import time
    import base64

    # Start the VM
    machine.start()
    machine.wait_for_unit("multi-user.target")

    # Wait for the kopia-exporter service to start
    machine.wait_for_unit("kopia-exporter.service")

    # Give the service a moment to bind to the port
    time.sleep(2)

    print("${testDescription}")

    # Test unauthenticated request - should fail with 401
    machine.fail("curl -f http://localhost:9090/metrics")

    # Verify 401 response with proper headers
    auth_response = machine.succeed("curl -s -w '%{http_code}' http://localhost:9090/metrics")
    assert "401" in auth_response

    # Test with correct credentials
    correct_auth = base64.b64encode(b"${expectedUser}:${expectedPass}").decode('ascii')
    metrics_output = machine.succeed(f"curl -f -H 'Authorization: Basic {correct_auth}' http://localhost:9090/metrics")

    # Verify the metrics contain expected Prometheus format
    assert "kopia_snapshots_by_retention" in metrics_output
    assert "kopia_snapshot_total_size_bytes" in metrics_output

    # Test with incorrect credentials
    wrong_auth = base64.b64encode(b"wrong:wrong").decode('ascii')
    machine.fail(f"curl -f -H 'Authorization: Basic {wrong_auth}' http://localhost:9090/metrics")

    # Test that root endpoint also requires auth
    machine.fail("curl -f http://localhost:9090/")
    machine.succeed(f"curl -f -H 'Authorization: Basic {correct_auth}' http://localhost:9090/")

    print("${testDescription} passed!")
  '';
in {
  # Test with username/password configuration
  userpass = pkgs.nixosTest {
    name = "kopia-exporter-auth-userpass";

    nodes.machine = {
      config,
      lib,
      pkgs,
      ...
    }:
      baseVmConfig
      // {
        networking.hostName = "kopia-exporter-userpass-test";

        # Enable the kopia-exporter service with username/password auth
        services.kopia-exporter = {
          enable = true;
          bind = "0.0.0.0:9090";
          kopiaBin = "${kopia-exporter}/bin/fake-kopia";
          auth = {
            enable = true;
            username = "testuser";
            password = "testpass";
          };
        };
      };

    testScript = commonAuthTestScript {
      expectedUser = "testuser";
      expectedPass = "testpass";
      testDescription = "Testing username/password authentication via NixOS module";
    };
  };

  # Test with credentials file configuration
  credfile = pkgs.nixosTest {
    name = "kopia-exporter-auth-credfile";

    nodes.machine = {
      config,
      lib,
      pkgs,
      ...
    }:
      baseVmConfig
      // {
        networking.hostName = "kopia-exporter-credfile-test";

        # Create credentials file
        environment.etc."kopia-exporter-creds".text = "fileuser:filepass";

        # Enable the kopia-exporter service with credentials file auth
        services.kopia-exporter = {
          enable = true;
          bind = "0.0.0.0:9090";
          kopiaBin = "${kopia-exporter}/bin/fake-kopia";
          auth = {
            enable = true;
            credentialsFile = "/etc/kopia-exporter-creds";
          };
        };
      };

    testScript = commonAuthTestScript {
      expectedUser = "fileuser";
      expectedPass = "filepass";
      testDescription = "Testing credentials file authentication via NixOS module";
    };
  };
}

{
  pkgs ? import <nixpkgs> {},
  system ? builtins.currentSystem,
  kopia-exporter ? pkgs.callPackage ../. {},
}: let
  vmTestLib = import ./vm-test-lib.nix {inherit pkgs kopia-exporter;};

  # Test authentication (requires auth config in service)
  testAuthentication = {
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
    assert "kopia_snapshot_size_bytes_total" in metrics_output

    # Test with incorrect credentials
    wrong_auth = base64.b64encode(b"wrong:wrong").decode('ascii')
    machine.fail(f"curl -f -H 'Authorization: Basic {wrong_auth}' http://localhost:${port}/metrics")

    # Test that root endpoint also requires auth
    machine.fail("curl -f http://localhost:${port}/")
    machine.succeed(f"curl -f -H 'Authorization: Basic {correct_auth}' http://localhost:${port}/")
  '';
in {
  # Test with username/password configuration
  userpass = pkgs.testers.nixosTest {
    name = "kopia-exporter-auth-userpass";

    nodes.machine = {
      config,
      lib,
      pkgs,
      ...
    }:
      vmTestLib.baseVmConfig
      // {
        networking.hostName = "kopia-exporter-userpass-test";

        # Enable the kopia-exporter service with username/password auth
        services.kopia-exporter = vmTestLib.mkServiceConfig {
          extraConfig = {
            auth = {
              enable = true;
              username = "testuser";
              password = "testpass";
            };
          };
        };
      };

    testScript = vmTestLib.mkTestScript {
      imports = ["time" "base64"];
      customTests = testAuthentication {
        user = "testuser";
        password = "testpass";
        description = "username/password authentication via NixOS module";
      };
      testDescription = "Username/password authentication test";
    };
  };

  # Test with credentials file configuration
  credfile = pkgs.testers.nixosTest {
    name = "kopia-exporter-auth-credfile";

    nodes.machine = {
      config,
      lib,
      pkgs,
      ...
    }:
      vmTestLib.baseVmConfig
      // {
        networking.hostName = "kopia-exporter-credfile-test";

        # Create credentials file
        environment.etc."kopia-exporter-creds".text = "fileuser:filepass";

        # Enable the kopia-exporter service with credentials file auth
        services.kopia-exporter = vmTestLib.mkServiceConfig {
          extraConfig = {
            auth = {
              enable = true;
              credentialsFile = "/etc/kopia-exporter-creds";
            };
          };
        };
      };

    testScript = vmTestLib.mkTestScript {
      imports = ["time" "base64"];
      customTests = testAuthentication {
        user = "fileuser";
        password = "filepass";
        description = "credentials file authentication via NixOS module";
      };
      testDescription = "Credentials file authentication test";
    };
  };
}

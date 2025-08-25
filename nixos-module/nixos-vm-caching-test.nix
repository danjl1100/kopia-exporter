{
  pkgs ? import <nixpkgs> {},
  system ? builtins.currentSystem,
  kopia-exporter ? pkgs.callPackage ../. {},
}: let
  vmTestLib = import ./vm-test-lib.nix {inherit pkgs kopia-exporter;};
  logPath = "/home/kopia-exporter/fake-kopia-test.log";
in
  pkgs.nixosTest {
    name = "kopia-exporter-caching";

    nodes.machine = {
      config,
      lib,
      pkgs,
      ...
    }:
      vmTestLib.baseVmConfig
      // {
        networking.hostName = "kopia-exporter-caching-test";

        # Enable the kopia-exporter service with caching test configuration
        services.kopia-exporter = vmTestLib.mkServiceConfig {
          extraConfig = {
            cacheSeconds = 1; # Short cache for faster testing
            environment = {
              FAKE_KOPIA_LOG = logPath;
            };
            # Override user home for testing
            user = "kopia-exporter";
          };
        };

        # Configure user with custom home for easier testing
        users.users.kopia-exporter = {
          home = lib.mkForce "/home/kopia-exporter";
          createHome = lib.mkForce true;
        };
      };

    testScript = vmTestLib.mkTestScript {
      customTests = vmTestLib.httpTests.caching {
        inherit logPath;
        cacheSeconds = 1;
        description = "caching behavior";
      };
      testDescription = "Caching behavior test";
    };
  }

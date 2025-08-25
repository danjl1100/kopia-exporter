{
  pkgs ? import <nixpkgs> {},
  system ? builtins.currentSystem,
  kopia-exporter ? pkgs.callPackage ../. {},
}: let
  vmTestLib = import ./vm-test-lib.nix {inherit pkgs kopia-exporter;};
in
  pkgs.nixosTest {
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
      customTests = vmTestLib.httpTests.basicEndpoints {};
      testDescription = "Basic service test";
    };
  }

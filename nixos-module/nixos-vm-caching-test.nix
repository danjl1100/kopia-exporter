{
  pkgs ? import <nixpkgs> {},
  system ? builtins.currentSystem,
  kopia-exporter ? pkgs.callPackage ../. {},
}: let
  vmTestLib = import ./vm-test-lib.nix {inherit pkgs kopia-exporter;};
  logPath = "/home/kopia-exporter/fake-kopia-test.log";

  # Test caching behavior
  testCaching = {
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
    call_count = len([line for line in log_content.strip().split('\n') if line.strip() == 'invocation, None'])
    assert call_count == 1, f"Expected 1 kopia call due to caching, got {call_count}. Log: {log_content}"

    # Wait for cache to expire and make another request
    time.sleep(${toString (cacheSeconds + 1)})
    machine.succeed("curl -f http://localhost:${port}/metrics")

    # Should now have 2 calls total
    log_content = machine.succeed("cat ${logPath}")
    call_count = len([line for line in log_content.strip().split('\n') if line.strip() == 'invocation, None'])
    assert call_count == 2, f"Expected 2 kopia calls after cache expiry, got {call_count}. Log: {log_content}"
  '';
in
  pkgs.testers.nixosTest {
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
      customTests = testCaching {
        inherit logPath;
        cacheSeconds = 1;
        description = "caching behavior";
      };
      testDescription = "Caching behavior test";
    };
  }

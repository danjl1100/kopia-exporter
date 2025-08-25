{
  pkgs ? import <nixpkgs> {},
  system ? builtins.currentSystem,
  kopia-exporter ? pkgs.callPackage ../. {},
}: let
  vmTestLib = import ./vm-test-lib.nix {inherit pkgs kopia-exporter;};
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
      customTests = vmTestLib.httpTests.authentication {
        user = "testuser";
        password = "testpass";
        description = "username/password authentication via NixOS module";
      };
      testDescription = "Username/password authentication test";
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
      customTests = vmTestLib.httpTests.authentication {
        user = "fileuser";
        password = "filepass";
        description = "credentials file authentication via NixOS module";
      };
      testDescription = "Credentials file authentication test";
    };
  };
}

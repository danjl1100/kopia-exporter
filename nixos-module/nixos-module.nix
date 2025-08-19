{
  config,
  lib,
  pkgs,
  ...
}:
with lib; let
  cfg = config.services.kopia-exporter;
in {
  options.services.kopia-exporter = {
    enable = mkEnableOption "Kopia Exporter service";

    package = mkOption {
      type = types.package;
      default = pkgs.callPackage ../. {};
      defaultText = literalExpression "pkgs.callPackage ../. { }";
      description = "The kopia-exporter package to use.";
    };

    kopiaBin = mkOption {
      type = types.str;
      default = "${pkgs.kopia}/bin/kopia";
      defaultText = literalExpression "\${pkgs.kopia}/bin/kopia";
      description = "Path to the kopia binary.";
    };

    bind = mkOption {
      type = types.str;
      default = "127.0.0.1:9090";
      description = "Address and port to bind the HTTP server to.";
    };

    user = mkOption {
      type = types.str;
      default = "kopia-exporter";
      description = "User account under which kopia-exporter runs.";
    };

    group = mkOption {
      type = types.str;
      default = "kopia-exporter";
      description = "Group account under which kopia-exporter runs.";
    };

    cacheSeconds = mkOption {
      type = types.ints.unsigned;
      default = 30;
      description = "Cache duration in seconds for kopia snapshot data (0 to disable).";
    };

    maxBindRetries = mkOption {
      type = types.ints.unsigned;
      default = 5;
      description = "Maximum number of bind retry attempts (0 = no retries, just 1 attempt).";
    };

    extraArgs = mkOption {
      type = types.listOf types.str;
      default = [];
      description = "Additional command line arguments to pass to kopia-exporter.";
    };

    environment = mkOption {
      type = types.attrsOf types.str;
      default = {};
      description = "Environment variables to set for the kopia-exporter service.";
    };

    after = mkOption {
      type = types.listOf types.str;
      default = ["network.target"];
      description = "Systemd units that this service should start after.";
    };

    bindsTo = mkOption {
      type = types.listOf types.str;
      default = [];
      description = "Systemd units that this service should bind to (stop when they stop).";
    };
  };

  config = mkIf cfg.enable {
    systemd.services.kopia-exporter = ({
      description = "Kopia Exporter - Prometheus metrics exporter for Kopia";
      wantedBy = ["multi-user.target"];
      after = cfg.after;

      serviceConfig =
        {
          Type = "simple";
          User = cfg.user;
          Group = cfg.group;
          Restart = "always";
          RestartSec = "10s";

          # Security hardening
          NoNewPrivileges = true;
          PrivateTmp = true;
          ProtectSystem = "strict";
          ProtectKernelTunables = true;
          ProtectKernelModules = true;
          ProtectControlGroups = true;
          RestrictRealtime = true;
          RestrictSUIDSGID = true;
          RemoveIPC = true;
          PrivateMounts = true;

          # Allow home directory access for child process (kopia cache and credentials)
          ProtectHome = false;
          # Allow write access to user home directory for kopia cache and logs
          ReadWritePaths = [config.users.users.${cfg.user}.home];
          # Allow network access for the HTTP server
          PrivateNetwork = false;

          # Memory and process limits
          MemoryHigh = "128M";
          MemoryMax = "256M";
          # Allow sufficient tasks for Go runtime (kopia subprocess needs multiple threads)
          TasksMax = 100;
        }
        // lib.optionalAttrs (cfg.environment != {}) {
          Environment = lib.mapAttrsToList (name: value: "${name}=${value}") cfg.environment;
        };

      script = ''
        exec ${cfg.package}/bin/kopia-exporter \
          --kopia-bin "${cfg.kopiaBin}" \
          --bind "${cfg.bind}" \
          --cache-seconds "${toString cfg.cacheSeconds}" \
          --max-bind-retries "${toString cfg.maxBindRetries}" \
          ${escapeShellArgs cfg.extraArgs}
      '';
    })
    // lib.optionalAttrs (cfg.bindsTo != []) {
      bindsTo = cfg.bindsTo;
    };

    users.users = mkIf (cfg.user == "kopia-exporter") {
      kopia-exporter = {
        description = "Kopia Exporter service user";
        group = cfg.group;
        home = "/var/lib/kopia-exporter";
        createHome = true;
        isSystemUser = true;
      };
    };

    users.groups = mkIf (cfg.group == "kopia-exporter") {
      kopia-exporter = {};
    };
  };
}

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
      default = pkgs.callPackage ./. {};
      defaultText = literalExpression "pkgs.callPackage ./. { }";
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

    extraArgs = mkOption {
      type = types.listOf types.str;
      default = [];
      description = "Additional command line arguments to pass to kopia-exporter.";
    };
  };

  config = mkIf cfg.enable {
    systemd.services.kopia-exporter = {
      description = "Kopia Exporter - Prometheus metrics exporter for Kopia";
      wantedBy = ["multi-user.target"];
      after = ["network.target"];

      serviceConfig = {
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
        # Allow network access for the HTTP server
        PrivateNetwork = false;

        # Memory and process limits
        MemoryHigh = "128M";
        MemoryMax = "256M";
        TasksMax = 10;
      };

      script = ''
        exec ${cfg.package}/bin/kopia-exporter \
          --kopia-bin "${cfg.kopiaBin}" \
          --bind "${cfg.bind}" \
          ${escapeShellArgs cfg.extraArgs}
      '';
    };

    users.users = mkIf (cfg.user == "kopia-exporter") {
      kopia-exporter = {
        description = "Kopia Exporter service user";
        group = cfg.group;
        isSystemUser = true;
      };
    };

    users.groups = mkIf (cfg.group == "kopia-exporter") {
      kopia-exporter = {};
    };
  };
}

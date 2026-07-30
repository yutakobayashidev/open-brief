{ defaultPackage }:

{
  config,
  lib,
  pkgs,
  ...
}:

let
  cfg = config.services.openbrief;
  toml = pkgs.formats.toml { };
  generatedConfig = toml.generate "openbrief-config.toml" cfg.settings;
in
{
  options.services.openbrief = {
    enable = lib.mkEnableOption "OpenBrief context recall";

    package = lib.mkOption {
      type = lib.types.package;
      default = defaultPackage pkgs.stdenv.hostPlatform.system;
      defaultText = lib.literalExpression "inputs.openbrief.packages.\${pkgs.system}.default";
      description = "The OpenBrief package to run.";
    };

    settings = lib.mkOption {
      type = toml.type;
      default = {
        retention_days = 7;
        capture.excluded_apps = [
          "1password"
          "com.1password.1password"
          "signal"
          "org.signal.signal"
          "discord"
          "com.discordapp.discord"
          "vesktop"
          "dev.vencord.vesktop"
        ];
      };
      example = lib.literalExpression ''
        {
          retention_days = 3;
          capture.excluded_apps = [
            "1password"
            "signal"
            "vesktop"
          ];
        }
      '';
      description = ''
        OpenBrief configuration written as TOML. Do not put credentials here:
        generated Nix store files are not a secret storage boundary.
      '';
    };
  };

  config = lib.mkIf cfg.enable {
    assertions = [
      {
        assertion = pkgs.stdenv.hostPlatform.isLinux;
        message = "services.openbrief is only supported on Linux.";
      }
    ];

    home.packages = [ cfg.package ];

    xdg.configFile."openbrief/config.toml".source = generatedConfig;

    systemd.user.services.openbrief = {
      Unit = {
        Description = "OpenBrief local daemon";
        Documentation = [ "https://github.com/yutakobayashidev/open-brief" ];
        After = [ "graphical-session.target" ];
        PartOf = [ "graphical-session.target" ];
        X-Restart-Triggers = [ generatedConfig ];
      };

      Service = {
        Type = "simple";
        ExecStart = "${lib.getExe' cfg.package "openbriefd"}";
        Restart = "on-failure";
        RestartSec = 3;
        UMask = "0077";
        NoNewPrivileges = true;
      };

      Install.WantedBy = [ "graphical-session.target" ];
    };
  };
}

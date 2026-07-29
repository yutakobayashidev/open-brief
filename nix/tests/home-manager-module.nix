{
  lib,
  module,
  pkgs,
  runCommand,
  writeShellScriptBin,
}:

let
  package = writeShellScriptBin "openbrief" "exit 0";
  evaluated = lib.evalModules {
    specialArgs = { inherit pkgs; };
    modules = [
      {
        options = {
          assertions = lib.mkOption {
            type = lib.types.listOf lib.types.unspecified;
            default = [ ];
          };
          home.packages = lib.mkOption {
            type = lib.types.listOf lib.types.package;
            default = [ ];
          };
          systemd.user.services = lib.mkOption {
            type = lib.types.attrs;
            default = { };
          };
          xdg.configFile = lib.mkOption {
            type = lib.types.attrs;
            default = { };
          };
        };
      }
      module
      {
        services.openbrief = {
          enable = true;
          inherit package;
          settings = {
            retention_days = 3;
            capture.excluded_apps = [
              "1password"
              "vesktop"
            ];
          };
        };
      }
    ];
  };
  assertionsPass = lib.all (assertion: assertion.assertion) evaluated.config.assertions;
  configSource = evaluated.config.xdg.configFile."openbrief/config.toml".source;
  service = evaluated.config.systemd.user.services.openbrief;
in
assert assertionsPass;
assert evaluated.config.home.packages == [ package ];
assert service.Unit.After == [ "graphical-session.target" ];
assert service.Unit.PartOf == [ "graphical-session.target" ];
assert service.Unit.X-Restart-Triggers == [ configSource ];
assert service.Service.ExecStart == "${lib.getExe' package "openbrief"} watch";
assert service.Service.UMask == "0077";
assert service.Service.NoNewPrivileges;
assert service.Install.WantedBy == [ "graphical-session.target" ];
runCommand "openbrief-home-manager-module-test" { } ''
  grep -F 'retention_days = 3' ${configSource}
  grep -F '"vesktop"' ${configSource}
  touch "$out"
''

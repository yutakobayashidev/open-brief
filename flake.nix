{
  description = "Local-first context recall for Linux and niri";

  inputs = {
    nixpkgs.url = "https://flakehub.com/f/NixOS/nixpkgs/0.1";
    fenix = {
      url = "https://flakehub.com/f/nix-community/fenix/0.1";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    { self, ... }@inputs:

    let
      developmentSystems = [
        "x86_64-linux"
        "aarch64-linux"
        "aarch64-darwin"
      ];
      linuxSystems = [
        "x86_64-linux"
        "aarch64-linux"
      ];
      forSystems =
        systems: f:
        inputs.nixpkgs.lib.genAttrs systems (
          system:
          f {
            inherit system;
            pkgs = import inputs.nixpkgs {
              inherit system;
              overlays = [ self.overlays.default ];
            };
          }
        );
      forDevelopmentSystems = forSystems developmentSystems;
      forLinuxSystems = forSystems linuxSystems;
      homeManagerModule = import ./nix/home-manager.nix {
        defaultPackage = system: self.packages.${system}.default;
      };
    in
    {
      overlays.default = final: _prev: {
        rustToolchain =
          with inputs.fenix.packages.${final.stdenv.hostPlatform.system};
          combine (
            with stable;
            [
              clippy
              rustc
              cargo
              rustfmt
              rust-src
            ]
          );

        openbrief = final.callPackage ./nix/package.nix {
          codexAcp = final.openbrief-codex-acp;
          src = self;
          rustPlatform = final.makeRustPlatform {
            cargo = final.rustToolchain;
            rustc = final.rustToolchain;
          };
        };

        openbrief-codex-acp = final.callPackage ./nix/codex-acp.nix { };
      };

      packages = forLinuxSystems (
        { pkgs, ... }:
        {
          default = pkgs.openbrief;
          inherit (pkgs) openbrief openbrief-codex-acp;
        }
      );

      apps = forLinuxSystems (
        { pkgs, ... }:
        {
          default = {
            type = "app";
            program = inputs.nixpkgs.lib.getExe pkgs.openbrief;
            meta.description = "Run the OpenBrief context recall CLI";
          };
          openbrief = self.apps.${pkgs.stdenv.hostPlatform.system}.default;
          desktop = {
            type = "app";
            program = inputs.nixpkgs.lib.getExe' pkgs.openbrief "openbrief-desktop";
            meta.description = "Run the OpenBrief desktop";
          };
        }
      );

      checks = forLinuxSystems (
        { pkgs, ... }:
        {
          package = pkgs.openbrief;
          home-manager-module = pkgs.callPackage ./nix/tests/home-manager-module.nix {
            module = homeManagerModule;
          };
        }
      );

      homeManagerModules = {
        default = homeManagerModule;
        openbrief = homeManagerModule;
      };

      # `homeModules` is the newer, shorter spelling used by some consumers.
      homeModules = self.homeManagerModules;

      devShells = forDevelopmentSystems (
        { pkgs, system }:
        {
          default = pkgs.mkShell {
            packages = with pkgs; [
              rustToolchain
              nodejs_24
              pnpm_10
              openssl
              pkg-config
              cairo
              gdk-pixbuf
              glib
              gtk3
              libsoup_3
              pango
              webkitgtk_4_1
              cargo-deny
              cargo-edit
              cargo-watch
              rust-analyzer
              self.formatter.${system}
            ];

            env.RUST_SRC_PATH = "${pkgs.rustToolchain}/lib/rustlib/src/rust/library";
          };
        }
      );

      formatter = forDevelopmentSystems ({ pkgs, ... }: pkgs.nixfmt);
    };
}

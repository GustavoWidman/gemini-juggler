{
  description = "gemini-juggler";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    crane.url = "github:ipetkov/crane";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs =
    {
      self,
      flake-utils,
      nixpkgs,
      fenix,
      crane,
      ...
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = nixpkgs.legacyPackages.${system};
        toolchain = fenix.packages.${system}.fromToolchainFile {
          file = ./rust-toolchain.toml;
          sha256 = "sha256-k5yIkqMgO2jRG53pNlnZ9Na4LTCSRefM0+YPPnCWMMA=";
        };
        craneLib = (crane.mkLib pkgs).overrideToolchain toolchain;

        args = {
          src = craneLib.cleanCargoSource ./.;
          strictDeps = true;

          nativeBuildInputs = [ ];
          buildInputs = [ ];
        };

        bin = craneLib.buildPackage (
          args
          // {
            cargoArtifacts = craneLib.buildDepsOnly args;
          }
        );
      in
      {
        checks.gemini-juggler = bin;

        packages.default = bin;
        apps.default = flake-utils.lib.mkApp { drv = bin; };

        devShells.default = pkgs.mkShell {
          buildInputs = [
            toolchain
          ];
        };
      }
    )
    // {
      nixosModules.default =
        {
          config,
          lib,
          pkgs,
          ...
        }:
        with lib;
        let
          cfg = config.services.gemini-juggler;
        in
        {
          options.services.gemini-juggler = {
            enable = lib.mkEnableOption "Enable gemini-juggler service";

            package = lib.mkOption {
              type = lib.types.package;
              default = self.packages.${pkgs.system}.default;
              description = "gemini-juggler package to use";
            };

            config = mkOption {
              type = types.nullOr types.path;
              default = null;
              example = "/etc/gemini-juggler/config.toml";
              description = "Path to config.toml file";
            };

            user = mkOption {
              type = types.str;
              default = "gemini-juggler";
              description = "User account under which gemini-juggler runs";
            };

            dataDir = mkOption {
              type = types.str;
              default = "/var/lib/gemini-juggler";
              description = "Directory where the database and state will be stored";
            };

            group = mkOption {
              type = types.str;
              default = "gemini-juggler";
              description = "Group under which gemini-juggler runs";
            };
          };

          config = lib.mkIf cfg.enable {
            users.users.${cfg.user} = {
              isSystemUser = true;
              group = cfg.group;
              home = cfg.dataDir;
              createHome = true;
            };

            users.groups.${cfg.group} = { };

            systemd.services.gemini-juggler = {
              description = "gemini-juggler service";
              wantedBy = [ "multi-user.target" ];
              after = [ "network.target" ];

              serviceConfig = {
                Type = "simple";
                User = cfg.user;
                Group = cfg.group;
                WorkingDirectory = cfg.dataDir;
                ExecStart = "${cfg.package}/bin/gemini-juggler";
                Restart = "on-failure";
                RestartSec = 5;

                NoNewPrivileges = true;
                PrivateTmp = true;
                ProtectSystem = "strict";
                ProtectHome = true;
                ReadWritePaths = [ cfg.dataDir ];
              };

              preStart = ''
                mkdir -p ${cfg.dataDir}

                ${optionalString (cfg.config != null) ''
                  ln -sf ${cfg.config} ${cfg.dataDir}/config.toml
                ''}
              '';
            };
          };
        };
    };
}

{
  description = "Valence coprocessor app";

  nixConfig.extra-experimental-features = "nix-command flakes";
  nixConfig.extra-substituters = "https://coffeetables.cachix.org";
  nixConfig.extra-trusted-public-keys = ''
    coffeetables.cachix.org-1:BCQXDtLGFVo/rTG/J4omlyP/jbtNtsZIKHBMTjAWt8g=
  '';

  inputs = {
    nixpkgs.url = "nixpkgs/nixos-25.05";
    
    flake-parts.url = "github:hercules-ci/flake-parts";
    fp-addons.url = "github:timewave-computer/flake-parts-addons";

    devshell.url = "github:numtide/devshell";

    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    sp1-nix.url = "github:timewave-computer/sp1.nix";
    crate2nix.url = "github:timewave-computer/crate2nix";
  };

  outputs = inputs@{ self, flake-parts, ... }:
    flake-parts.lib.mkFlake {inherit inputs;} ({lib, ...}: {
      debug = true;
      imports = [
        inputs.devshell.flakeModule
        inputs.crate2nix.flakeModule
        inputs.fp-addons.flakeModules.tools
      ];

      systems = ["x86_64-linux" "aarch64-linux" "x86_64-darwin" "aarch64-darwin"];

      perSystem = {
        config,
        inputs',
        pkgs,
        system,
        valence,
        ...
      }: {
        _module.args.pkgs = import inputs.nixpkgs {
          inherit system;
          overlays = [
            inputs.rust-overlay.overlays.default
            (final: prev: {
              rustWithWasm = pkgs.rust-bin.nightly.latest.default.override {
                targets = [ "wasm32-unknown-unknown" ];
              };
            })
          ];
        };
        
        _module.args.valence = {
          toml = builtins.fromTOML (lib.readFile ./valence.toml);
          controllers = lib.attrValues (lib.mapAttrs (_: v: v.controller) valence.toml.circuit);
          circuits = lib.attrValues (lib.mapAttrs (_: v: v.circuit) valence.toml.circuit);
        };

        crate2nix = {
          devshell.name = "default";
        };
        # Separate crate2nix build for cross compiling
        # to not interfere with any main builds for executables
        crate2nix = {
          cargoNix = ./Cargo.nix;
          toolchain = {
            rust = pkgs.rustWithWasm;
            cargo = pkgs.rustWithWasm;
          };
          crateOverrides =
            let
              sp1Tools = inputs'.sp1-nix.tools;
              sp1Overrides = sp1Tools.crateOverrides;
              crate2nixOverrides = inputs'.crate2nix.tools.crateOverrides;
            in
            sp1Overrides // sp1Tools.sp1ElfCrateOverrides
              // (lib.genAttrs valence.circuits (_: sp1Overrides.sp1-elf-crate))
              // (lib.genAttrs valence.controllers (_: crate2nixOverrides.wasm-crate));
        };

        packages = lib.getAttrs
          (valence.circuits ++ valence.controllers)
          config.crate2nix.packages;

        apps.default.program = pkgs.writeShellScriptBin "build-circuits" ''
          ${lib.concatStringsSep "\n" (lib.mapAttrsToList (name: circuit: ''
            mkdir -p ${valence.toml.valence.artifacts}/${name}
            nix build '.#${circuit.circuit}'
            ${pkgs.coreutils}/bin/install --mode=644 result/bin/* ${valence.toml.valence.artifacts}/${name}/circuit.bin
            rm result

            nix build '.#${circuit.controller}'
            ${pkgs.coreutils}/bin/install --mode=644 result/* ${valence.toml.valence.artifacts}/${name}/controller.bin
            rm result
          '') valence.toml.circuit)}
        '';

        devshells.default = {
          packages = with pkgs; [
            curl
            jq
            clang
            taplo
            toml-cli
            lld
          ];
          
          env = [
            {
              name = "OPENSSL_DIR";
              value = "${pkgs.lib.getDev pkgs.openssl}";
            }
            {
              name = "OPENSSL_LIB_DIR";
              value = "${pkgs.lib.getLib pkgs.openssl}/lib";
            }
            {
              name = "LIBCLANG_PATH";
              value = pkgs.lib.makeLibraryPath [ pkgs.libclang ];
            }
          ];
        };
      };
    });
}

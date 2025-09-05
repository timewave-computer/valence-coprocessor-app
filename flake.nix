{
  description = "Valence coprocessor app";

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
            set -e
            nix develop ''${NIX_ARGS:+$NIX_ARGS} --command update-cargo-nix
            # check if x86_64-linux packages can be built
            if nix build --impure --inputs-from . --expr \
              'let pkgs = import (builtins.getFlake "nixpkgs") { system = "x86_64-linux"; }; in
                pkgs.writeText "test-system" (toString builtins.currentTime)' 2>/dev/null
            then
              nix build ''${NIX_ARGS:+$NIX_ARGS} '.#packages.x86_64-linux.${circuit.circuit}' '.#packages.x86_64-linux.${circuit.controller}'
              mkdir -p ${valence.toml.valence.artifacts}/${name}
              install --mode=644 result/bin/* ${valence.toml.valence.artifacts}/${name}/circuit.bin
              install --mode=644 result-1/* ${valence.toml.valence.artifacts}/${name}/controller.bin
            else
              if docker info >/dev/null 2>/dev/null; then
                ENGINE=docker
                echo "Setting up x86_64-linux emulation with docker"
              else
                ENGINE=${pkgs.podman}/bin/podman
                echo "Setting up x86_64-linux emulation with podman (docker unavailable)"
                podman machine init || true
                podman machine start 2>/dev/null || true
              fi

              if $ENGINE image inspect nix-circuit-builder >/dev/null 2>&1; then
                echo Loading existing builder image: nix-circuit-builder
                $ENGINE create --name nix-circuit-builder --platform linux/amd64 -v "$(pwd)":/code -w /code -ti nix-circuit-builder bash
              else 
                $ENGINE create --name nix-circuit-builder \
                  --platform linux/amd64 -v "$(pwd)":/code -w /code -ti nixpkgs/nix-flakes sh -c \
                  "echo filter-syscalls = false >> /etc/nix/nix.conf && git config --global --add safe.directory '*' && exec bash"
              fi
              $ENGINE start nix-circuit-builder
              function cleanup {
                echo "Saving build state to image: nix-circuit-builder"
                $ENGINE commit nix-circuit-builder nix-circuit-builder
                $ENGINE stop nix-circuit-builder
                $ENGINE rm nix-circuit-builder
              }
              trap cleanup EXIT
              PREFIX="$ENGINE exec -t nix-circuit-builder"
              $PREFIX nix build ''${NIX_ARGS:+$NIX_ARGS} '.#${circuit.circuit}' '.#${circuit.controller}'
              mkdir -p ${valence.toml.valence.artifacts}/${name}
              $PREFIX sh -c "install --mode=644 result/bin/* ${valence.toml.valence.artifacts}/${name}/circuit.bin"
              $PREFIX sh -c "install --mode=644 result-1/* ${valence.toml.valence.artifacts}/${name}/controller.bin"
            fi
            rm -f result result-1
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
            cargo
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

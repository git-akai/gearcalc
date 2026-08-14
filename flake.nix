{
  description = "Gear and geartrain design tool — Rust core, WebAssembly boundary, Svelte UI";

  # Derived from schlarpc/rust-flake. Deviations, all deliberate:
  #   - workspace rather than a single crate
  #   - wasm32-unknown-unknown instead of the Windows MSVC cross-compilation
  #     target (this ships as a web application; a Windows .exe of the CLI is not
  #     a goal, and dropping the xwin fixed-output derivation removes a pinned
  #     CRT/SDK hash that would need maintaining)
  #   - nodejs in the dev shell for the Svelte front end

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";

    systems.url = "github:nix-systems/default";

    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    crane.url = "github:ipetkov/crane";
  };

  outputs = { self, nixpkgs, systems, rust-overlay, crane, ... }:
    let
      eachSystem = nixpkgs.lib.genAttrs (import systems);

      pkgsFor = system: import nixpkgs {
        inherit system;
        overlays = [ rust-overlay.overlays.default ];
      };

      # Toolchain pinned via rust-toolchain.toml (single source of truth), so
      # rustup users outside Nix get the same version.
      rustToolchainFor = system:
        (pkgsFor system).rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;

      craneLibFor = system:
        (crane.mkLib (pkgsFor system)).overrideToolchain (rustToolchainFor system);

      # Crane's `cleanCargoSource` keeps only what Cargo itself needs — .rs,
      # Cargo.toml, Cargo.lock. gear-core also `include_str!`s a data file (the
      # JGMA tolerance tables, deliberately kept as a flat CSV so it can be
      # diffed against the standard), and that would be filtered out, so the
      # crate compiles from a checkout but not from the Nix sandbox. Keep the
      # data files too.
      #
      # Anything else added under a `data/` directory is picked up automatically;
      # a new file *type* elsewhere would need adding here.
      srcFor = system:
        let
          pkgs = pkgsFor system;
          craneLib = craneLibFor system;
        in
        pkgs.lib.cleanSourceWith {
          src = ./.;
          name = "source";
          filter = path: type:
            (craneLib.filterCargoSources path type)
            || (builtins.match ".*/data/[^/]*\\.csv$" path != null);
        };

      commonArgsFor = system:
        {
          src = srcFor system;
          strictDeps = true;
          # The workspace root has no [package], so crane cannot infer these.
          # Setting them names the derivations usefully; it does not silence
          # crane's eval-time "placeholder value" warning, which is emitted
          # while inspecting the root manifest and is cosmetic.
          pname = "gears";
          version = "0.1.0";
          buildInputs = [ ];
          nativeBuildInputs = [ ];
        };

      cargoArtifactsFor = system:
        (craneLibFor system).buildDepsOnly (commonArgsFor system);

      # --- the browser build ---------------------------------------------
      #
      # Two stages, because the front end consumes the Rust core as an artifact:
      #   1. compile gear-wasm to wasm32 and run wasm-bindgen over it
      #   2. build the Svelte app with those bindings already in place
      #
      # `wasm-bindgen-cli` here and the `wasm-bindgen` crate pinned in
      # crates/gear-wasm/Cargo.toml must agree on the bindgen format. If a
      # nixpkgs bump changes the CLI, that pin is what to update.
      wasmBindingsFor = system:
        let
          pkgs = pkgsFor system;
          craneLib = craneLibFor system;
          commonArgs = commonArgsFor system;
        in
        craneLib.buildPackage (commonArgs // {
          pname = "gear-wasm-bindings";
          cargoExtraArgs = "--package gear-wasm";
          CARGO_BUILD_TARGET = "wasm32-unknown-unknown";
          doCheck = false; # wasm cannot run on the build host

          nativeBuildInputs = commonArgs.nativeBuildInputs ++ [ pkgs.wasm-bindgen-cli ];

          installPhaseCommand = ''
            wasm-bindgen --target web --out-dir "$out" \
              target/wasm32-unknown-unknown/release/gear_wasm.wasm
          '';
        });

      webFor = system:
        let
          pkgs = pkgsFor system;
        in
        pkgs.buildNpmPackage {
          pname = "gears-web";
          version = "0.1.0";
          src = ./web;
          npmDepsHash = "sha256-jFq5QcK1NF5FQJA5Sv3e0/WYNAdhsX6beOu7c47CMsI=";

          # The wasm stage already ran; skip the npm script that would rerun it
          # (cargo cannot reach the network inside the sandbox anyway).
          preBuild = ''
            mkdir -p src/wasm
            cp -r ${wasmBindingsFor system}/* src/wasm/
          '';
          buildPhase = ''
            runHook preBuild
            npx vite build
            runHook postBuild
          '';
          installPhase = ''
            runHook preInstall
            cp -r dist "$out"
            runHook postInstall
          '';
        };
    in
    {
      packages = eachSystem (system:
        let
          craneLib = craneLibFor system;
          commonArgs = commonArgsFor system;
        in
        {
          default = craneLib.buildPackage (commonArgs // {
            cargoArtifacts = cargoArtifactsFor system;
            doCheck = false; # tests run in `checks`, not during build
          });

          # The deployable static site: `nix build .#web` -> ./result
          web = webFor system;

          # The wasm-bindgen output on its own, useful for debugging the boundary.
          wasm = wasmBindingsFor system;
        });

      checks = eachSystem (system:
        let
          craneLib = craneLibFor system;
          commonArgs = commonArgsFor system;
          cargoArtifacts = cargoArtifactsFor system;
        in
        {
          build = self.packages.${system}.default;

          clippy = craneLib.cargoClippy (commonArgs // {
            inherit cargoArtifacts;
            cargoClippyExtraArgs = "--all-targets -- --deny warnings";
          });

          fmt = craneLib.cargoFmt { src = commonArgs.src; };

          test = craneLib.cargoNextest (commonArgs // {
            inherit cargoArtifacts;
            partitions = 1;
            partitionType = "count";
          });
        });

      devShells = eachSystem (system:
        let pkgs = pkgsFor system;
        in {
          default = pkgs.mkShell {
            nativeBuildInputs = [
              (rustToolchainFor system)   # includes rust-analyzer, rustfmt, clippy
              pkgs.cargo-nextest          # fast parallel test runner
              pkgs.cargo-llvm-cov         # coverage
              pkgs.bacon                  # watch mode
              pkgs.cargo-edit
              pkgs.wasm-bindgen-cli       # Rust -> wasm boundary
              pkgs.binaryen               # wasm-opt
              pkgs.nodejs                 # Svelte / Vite front end

              # ezdxf reads exported DXF back with an implementation unrelated
              # to ours, so tools/validate_dxf.py checks the geometry rather
              # than only our agreement with ourselves.
              (pkgs.python3.withPackages (ps: [ ps.ezdxf ]))
            ];

            RUST_BACKTRACE = "1";
          };
        });
    };
}

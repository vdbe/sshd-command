{
  description = "Development environment for sshd-command.";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
  };

  outputs =
    {
      self,
      nixpkgs,
    }:
    let
      inherit (nixpkgs) lib;

      # We want to generate outputs for as many systems as possible,
      # even if we don't officially support or test for them
      allSystems = lib.systems.flakeExposed;

      # These are the systems we do officially support and test, though
      supportedSystems = [
        "x86_64-linux"
        "aarch64-linux"
        # "x86_64-darwin"
        # "aarch64-darwin"
      ];

      forAllSystems = lib.genAttrs allSystems;
      nixpkgsFor = nixpkgs.legacyPackages;
    in
    {

      checks = forAllSystems (
        system:
        let
          pkgs = nixpkgsFor.${system};
          packages = self.packages.${system};

          mkCheck =
            name: deps: script:
            pkgs.runCommand name { nativeBuildInputs = deps; } ''
              ${script}
              touch $out
            '';
        in
        lib.optionalAttrs (lib.elem system supportedSystems) {
          package_sshd-command = packages.default;

          clippy = (packages.default.override { lto = false; }).overrideAttrs {
            pname = "check-clippy";

            nativeBuildInputs = [
              pkgs.cargo
              pkgs.clippy
              pkgs.clippy-sarif
              pkgs.rustPlatform.cargoSetupHook
              pkgs.rustc
              pkgs.sarif-fmt
            ];

            buildPhase = ''
              runHook preBuild
              cargo clippy \
                --all-features \
                --all-targets \
                --tests \
                --message-format=json \
              | clippy-sarif | tee $out | sarif-fmt
              runHook postBuild
            '';

            dontInstall = true;
            doCheck = false;
            doInstallCheck = false;
            dontFixup = true;

            passthru = { };
            meta = { };
          };

          rustfmt = mkCheck "check-cargo-fmt" [
            pkgs.cargo
            pkgs.rustfmt
          ] "cd ${self} && cargo fmt -- --check";

          actionlint = mkCheck "check-actionlint" [
            pkgs.actionlint
          ] "actionlint ${self}/.github/workflows/*";

          deadnix = mkCheck "check-deadnix" [ pkgs.deadnix ] "deadnix --fail ${self}";

          nixfmt = mkCheck "check-nixfmt" [ pkgs.nixfmt-rfc-style ] "nixfmt --check ${self}";

          statix = mkCheck "check-statix" [ pkgs.statix ] "statix check ${self}";
        }
      );

      devShells = forAllSystems (
        system:
        let
          pkgs = nixpkgsFor.${system};
        in
        lib.optionalAttrs (lib.elem system supportedSystems) {
          default = pkgs.mkShell {
            packages = [
              # Rust tools
              pkgs.clippy
              pkgs.clippy-sarif
              pkgs.sarif-fmt
              pkgs.rust-analyzer
              pkgs.rustfmt
              pkgs.cargo-expand

              # Nix tools
              self.formatter.${system}
              pkgs.nixd
              pkgs.statix

              # Github action tools
              pkgs.efm-langserver
              # pkgs.yaml-language-server
              pkgs.actionlint
            ];

            env = {
              RUST_SRC_PATH = toString pkgs.rustPlatform.rustLibSrc;
            };

            inputsFrom = [ self.packages.${system}.default ];
          };
        }
      );

      formatter = forAllSystems (system: nixpkgsFor.${system}.nixfmt-rfc-style);

      packages = forAllSystems (
        system:
        let
          pkgs = nixpkgsFor.${system};
          pkgs' = import ./default.nix { inherit pkgs; };

          static-x86_64-pkgs' = import ./default.nix { pkgs = pkgs.pkgsCross.musl64.pkgsStatic; };
          static-aarch64-pkgs' = import ./default.nix {
            pkgs = pkgs.pkgsCross.aarch64-multiplatform.pkgsStatic;
          };

        in
        pkgs'
        // {
          default = pkgs'.sshd-command;

          static-x86_64 = static-x86_64-pkgs'.sshd-command;
          static-aarch64 = static-aarch64-pkgs'.sshd-command;
        }
      );
    };
}

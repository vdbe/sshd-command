{
  lib,
  stdenv,
  removeReferencesTo,
  rustPlatform,
  upx,

  lto ? true,
  optimizeSize ? stdenv.hostPlatform.isStatic,
  optimizeWithUpx ? false,
}:
let
  fs = lib.fileset;
in
rustPlatform.buildRustPackage rec {
  pname = passthru.cargoTOML.package.name;
  inherit (passthru.cargoTOML.package) version;

  src = fs.toSource {
    root = ../.;
    fileset = fs.intersection (fs.gitTracked ../.) (
      fs.unions [
        ../Cargo.lock
        ../Cargo.toml
        ../src

        ../rustfmt.toml
      ]
    );
  };

  cargoLock.lockFile = ../Cargo.lock;

  nativeBuildInputs =
    (lib.optional stdenv.hostPlatform.isStatic removeReferencesTo)
    ++ (lib.optional optimizeWithUpx upx);

  # `-C panic="abort"` breaks checks
  doCheck = !optimizeSize;

  postFixup = toString [
    (lib.optionalString stdenv.hostPlatform.isStatic ''
      find "$out" \
        -type f \
        -exec remove-references-to -t ${stdenv.cc.libc} '{}' +
    '')
    (lib.optionalString optimizeWithUpx ''
      upx --best --lzma "$out/bin/sshd-command"
    '')
  ];

  env =
    let
      rustFlags =
        lib.optionalAttrs lto {
          lto = "fat";
          embed-bitcode = "yes";
        }
        // lib.optionalAttrs optimizeSize {
          codegen-units = 1;
          opt-level = "s";
          panic = "abort";
          strip = "symbols";
        };
    in
    {
      RUSTFLAGS = toString (lib.mapAttrsToList (name: value: "-C ${name}=${toString value}") rustFlags);
    };

  passthru = {
    cargoTOML = lib.importTOML ../Cargo.toml;
  };

  meta = {
    license = lib.licenses.mit;
    mainProgram = "sshd-command";
  };
}

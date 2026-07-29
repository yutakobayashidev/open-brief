{
  lib,
  makeWrapper,
  rustPlatform,
  src,
  systemd,
}:

rustPlatform.buildRustPackage {
  pname = "openbrief";
  version = "0.1.0";
  inherit src;

  cargoLock.lockFile = "${src}/Cargo.lock";
  cargoBuildFlags = [
    "--package"
    "openbrief-cli"
  ];
  cargoTestFlags = [ "--workspace" ];

  nativeBuildInputs = [ makeWrapper ];

  postFixup = ''
    wrapProgram "$out/bin/openbrief" \
      --prefix PATH : ${lib.makeBinPath [ systemd ]}
  '';

  meta = {
    description = "Local-first context recall for Linux and niri";
    homepage = "https://github.com/yutakobayashidev/open-brief";
    license = lib.licenses.mit;
    mainProgram = "openbrief";
    platforms = lib.platforms.linux;
  };
}

{
  buildNpmPackage,
  fetchFromGitHub,
  lib,
  makeWrapper,
  nodejs_24,
  piCodingAgent,
}:

buildNpmPackage {
  pname = "openbrief-pi-acp";
  version = "0.0.33";

  src = fetchFromGitHub {
    owner = "svkozak";
    repo = "pi-acp";
    tag = "v0.0.33";
    hash = "sha256-fENOOdooi4XbIDjcr02q8qzUCzdo2IW/Bca43SawZ44=";
  };

  nodejs = nodejs_24;
  npmDepsHash = "sha256-/fX79XucKojL/6gZbK5eizEfrXso8rlTgiHfJffmDuY=";
  npmFlags = [ "--ignore-scripts" ];
  nativeBuildInputs = [ makeWrapper ];

  postFixup = ''
    wrapProgram "$out/bin/pi-acp" \
      --set PI_ACP_PI_COMMAND ${lib.getExe piCodingAgent}
  '';

  meta = {
    description = "ACP adapter for Pi, pinned for OpenBrief";
    homepage = "https://github.com/svkozak/pi-acp";
    license = lib.licenses.mit;
    mainProgram = "pi-acp";
    platforms = lib.platforms.linux;
  };
}

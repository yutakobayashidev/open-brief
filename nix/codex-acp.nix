{
  buildNpmPackage,
  fetchFromGitHub,
  lib,
  nodejs_24,
}:

buildNpmPackage {
  pname = "openbrief-codex-acp";
  version = "1.1.7";

  src = fetchFromGitHub {
    owner = "agentclientprotocol";
    repo = "codex-acp";
    tag = "v1.1.7";
    hash = "sha256-RY1iiajNR3eJI9WYARZnbIHnDl5+gmlPo3GVjJEJ9Zs=";
  };

  nodejs = nodejs_24;
  npmDepsHash = "sha256-c/sbGziA3Y2mOcPRD3K0PSd8sAVXSQuip8fE/eojl+Y=";
  npmFlags = [ "--ignore-scripts" ];

  meta = {
    description = "ACP adapter for the Codex CLI, pinned for OpenBrief";
    homepage = "https://github.com/agentclientprotocol/codex-acp";
    license = lib.licenses.asl20;
    mainProgram = "codex-acp";
    platforms = lib.platforms.linux;
  };
}

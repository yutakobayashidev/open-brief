{
  cairo,
  codexAcp,
  dbus,
  fetchPnpmDeps,
  gdk-pixbuf,
  glib,
  glib-networking,
  gtk3,
  lib,
  libsoup_3,
  makeWrapper,
  nodejs,
  pnpm_10,
  pnpmConfigHook,
  pkg-config,
  rustPlatform,
  src,
  systemd,
  webkitgtk_4_1,
  wrapGAppsHook3,
}:

rustPlatform.buildRustPackage (finalAttrs: {
  pname = "openbrief";
  version = "0.1.0";
  inherit src;

  cargoLock.lockFile = "${src}/Cargo.lock";
  cargoBuildFlags = [
    "--package"
    "openbrief-cli"
    "--package"
    "openbrief-desktop"
    "--features"
    "openbrief-desktop/custom-protocol"
  ];
  cargoTestFlags = [
    "--package"
    "openbrief-core"
    "--package"
    "openbrief-store"
    "--package"
    "openbrief-source-niri"
    "--package"
    "openbrief-app"
    "--package"
    "openbrief-agent"
    "--package"
    "openbrief-cli"
  ];

  pnpmDeps = fetchPnpmDeps {
    pname = "${finalAttrs.pname}-desktop";
    inherit (finalAttrs) version;
    pnpm = pnpm_10;
    src = "${finalAttrs.src}/apps/desktop";
    sourceRoot = "desktop";
    fetcherVersion = 3;
    hash = "sha256-G7gO3+13RhC5SrMdUNVI6FJthS87RUnV+g/L1dsxBro=";
  };
  pnpmRoot = "apps/desktop";

  nativeBuildInputs = [
    makeWrapper
    nodejs
    pnpm_10
    pnpmConfigHook
    pkg-config
    wrapGAppsHook3
  ];

  buildInputs = [
    cairo
    dbus
    gdk-pixbuf
    glib
    glib-networking
    gtk3
    libsoup_3
    webkitgtk_4_1
  ];

  preBuild = ''
    pnpm --dir apps/desktop run build
  '';

  postInstall = ''
    install -Dm644 apps/desktop/openbrief.desktop \
      "$out/share/applications/openbrief.desktop"
    install -Dm644 apps/desktop/src-tauri/icons/icon.svg \
      "$out/share/icons/hicolor/scalable/apps/openbrief.svg"
    mkdir -p "$out/libexec/openbrief"
    ln -s ${lib.getExe codexAcp} "$out/libexec/openbrief/codex-acp"
  '';

  preFixup = ''
    gappsWrapperArgs+=(
      --prefix LD_LIBRARY_PATH :
      ${lib.makeLibraryPath [
        cairo
        dbus
        gdk-pixbuf
        glib
        gtk3
        libsoup_3
        webkitgtk_4_1
      ]}
    )
  '';

  postFixup = ''
    wrapProgram "$out/bin/openbrief" \
      --prefix PATH : ${lib.makeBinPath [ systemd ]}
  '';

  meta = {
    description = "Local-first attention handoff and context recall";
    homepage = "https://github.com/yutakobayashidev/open-brief";
    license = lib.licenses.mit;
    mainProgram = "openbrief";
    platforms = lib.platforms.linux;
  };
})

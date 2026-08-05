{
  lib,
  stdenv,
  rustPlatform,

  fetchYarnDeps,
  yarnConfigHook,
  nodejs,

  pkg-config,
  wrapGAppsHook4,

  gtk3,
  webkitgtk_4_1,
  libsoup_3,
  glib,
  glib-networking,
  openssl,
  libayatana-appindicator,
  librsvg,

  src,
}:

rustPlatform.buildRustPackage {
  pname = "endfield";
  version = "0.3.0";

  inherit src;

  strictDeps = true;

  cargoRoot = "src-tauri";
  buildAndTestSubdir = "src-tauri";

  cargoLock = {
    lockFile = ./src-tauri/Cargo.lock;
  };

  yarnOfflineCache = fetchYarnDeps {
    yarnLock = ./yarn.lock;
    hash = "sha256-Se4javzX1QMvN3m+CzfRw99Tpc+AbC6s5gQRnyzjauE=";
  };

  nativeBuildInputs = [
    nodejs
    yarnConfigHook

    pkg-config
    wrapGAppsHook4
  ];

  buildInputs = [
    gtk3
    webkitgtk_4_1
    libsoup_3
    glib
    glib-networking
    openssl
    libayatana-appindicator
    librsvg
  ];

  preBuild = ''
    yarn --offline build
  '';

  doCheck = false;

  installPhase = ''
    runHook preInstall

    install -Dm755 \
      target/${stdenv.hostPlatform.rust.rustcTarget}/release/llauncher \
      $out/bin/endfield

    install -Dm644 \
      src-tauri/icons/32x32.png \
      $out/share/icons/hicolor/32x32/apps/endfield.png

    install -Dm644 \
      src-tauri/icons/128x128.png \
      $out/share/icons/hicolor/128x128/apps/endfield.png

    install -Dm644 \
      src-tauri/icons/128x128@2x.png \
      $out/share/icons/hicolor/256x256/apps/endfield.png

    install -Dm644 /dev/stdin \
      $out/share/applications/endfield.desktop <<'EOF'
[Desktop Entry]
Type=Application
Name=Endfield
Comment=Unofficial launcher for Arknights: Endfield
Exec=endfield
Icon=endfield
Terminal=false
Categories=Game;
StartupNotify=true
StartupWMClass=LLauncher
EOF

    runHook postInstall
  '';

  meta = {
    description = "Unofficial launcher for Arknights: Endfield on Linux";
    homepage = "https://github.com/thisislunacy/endfield";
    license = lib.licenses.mit;
    mainProgram = "endfield";
    platforms = [ "x86_64-linux" ];
  };
}

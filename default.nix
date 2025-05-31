{
  lib,
  rustPlatform,
  fetchFromGitHub,
  pkg-config,
  openssl,
  udev,
  vulkan-loader,
  stdenv,
  alsa-lib,
  pkgs,
}: let
  deps = with pkgs; ([
      openssl
    ]
    ++ [
      xorg.libX11.dev
      xorg.libXrandr.dev
      xorg.libXcursor.dev
      xorg.libXinerama.dev
      xorg.libXi.dev
      xorg.libXxf86vm.dev
      libxkbcommon
      udev
      alsa-lib-with-plugins
      vulkan-loader
      wayland
      git
    ]);
in
  rustPlatform.buildRustPackage rec {
    pname = "tasogare";
    version = "0-unstable-2025-05-31";

    src = fetchFromGitHub {
      owner = "make-42";
      repo = "tasogare";
      rev = "1adf781fce5f280839598fb0d27f5123d3a7facd";
      hash = "sha256-Y10GL/MyjBpcOrxLsUF6bCW03SODsR7/FB5XBkebz0s=";
    };

    cargoHash = "sha256-55ly1c+OZ5NlKXQXpDaSXyr4oRcegKSR1VBuuD+2Vrg=";

    nativeBuildInputs =
      [
        pkg-config
        rustPlatform.bindgenHook
        pkgs.makeWrapper
      ]
      ++ deps;

    buildInputs =
      [
        openssl
        udev
        vulkan-loader
      ]
      ++ lib.optionals stdenv.isLinux [
        alsa-lib
      ]
      ++ deps;

    meta = {
      description = "A satellite tracker";
      homepage = "https://github.com/make-42/tasogare";
      license = lib.licenses.mit;
      maintainers = with lib.maintainers; [];
      mainProgram = "tasogare";
    };

    postInstall = ''
      # Install assets
      mkdir -p $out/bin/assets
      cp -r ./assets/* $out/bin/assets/

      # Rename original compiled binary
      mv $out/bin/tasogare $out/bin/.tasogare-wrapped

      # Create a wrapper with correct LD_LIBRARY_PATH
      makeWrapper ${pkgs.lib.getBin pkgs.coreutils}/bin/env $out/bin/tasogare \
        --set LD_LIBRARY_PATH ${pkgs.lib.makeLibraryPath deps} \
        --add-flags "$out/bin/.tasogare-wrapped"
    '';
  }

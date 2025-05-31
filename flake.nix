{
  description = "Tasogare: A satellite tracker built with Rust";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = {
    self,
    nixpkgs,
    flake-utils,
  }:
    flake-utils.lib.eachDefaultSystem (system: let
      pkgs = import nixpkgs {
        inherit system;
      };

      tasogare = pkgs.callPackage ./default.nix {
        pkgs = pkgs;
      };

      deps = with pkgs;
        [
          cargo
          rustc
          bacon
          pkg-config
          openssl
          udev
          vulkan-loader
          alsa-lib
        ]
        ++ (with pkgs.xorg; [
          libX11.dev
          libXrandr.dev
          libXcursor.dev
          libXinerama.dev
          libXi.dev
          libXxf86vm.dev
        ])
        ++ [
          pkgs.libxkbcommon
          pkgs.wayland
        ];
    in {
      packages.default = tasogare;

      devShells.default = pkgs.mkShell {
        packages = deps;
        LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath deps;
      };
    });
}

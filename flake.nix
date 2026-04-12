{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, nixpkgs, flake-utils, rust-overlay }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs { inherit system overlays; };
        rustToolchain = pkgs.rust-bin.stable.latest.default;

        nativeBuildInputs = with pkgs; [
          rustToolchain
          pkg-config
          cmake
          clang
          makeWrapper
        ];

        buildInputs = with pkgs; [
          # Bevy / rendering
          vulkan-loader
          vulkan-headers
          vulkan-validation-layers

          # X11
          libx11
          libxcursor
          libxi
          libxrandr
          libxcb

          # Wayland
          wayland
          libxkbcommon

          # Audio (bevy dep even if unused)
          alsa-lib

          # System
          udev

          # rfd file dialogs (GTK backend)
          gtk3

          # spglib C library
          libclang
        ];

        runtimeLibs = with pkgs; [
          vulkan-loader
          wayland
          libxkbcommon
          libx11
          libxcursor
          libxi
          libxrandr
        ];

      in {
        devShells.default = pkgs.mkShell {
          inherit nativeBuildInputs buildInputs;

          LIBCLANG_PATH = "${pkgs.libclang.lib}/lib";
          CMAKE_POLICY_VERSION_MINIMUM = "3.5";

          LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath runtimeLibs;
        };

        packages.default = pkgs.rustPlatform.buildRustPackage {
          pname = "alfred";
          version = "0.2.0";
          src = ./.;
          cargoLock.lockFile = ./Cargo.lock;

          inherit nativeBuildInputs buildInputs;

          LIBCLANG_PATH = "${pkgs.libclang.lib}/lib";
          CMAKE_POLICY_VERSION_MINIMUM = "3.5";

          postFixup = ''
            wrapProgram $out/bin/alfred \
              --prefix LD_LIBRARY_PATH : ${pkgs.lib.makeLibraryPath runtimeLibs}
          '';
        };
      });
}

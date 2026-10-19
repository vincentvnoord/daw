{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { nixpkgs, rust-overlay, flake-utils, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [ rust-overlay.overlays.default ];
        };
        rust = pkgs.rust-bin.stable.latest.default.override {
          extensions = [ "rust-src" "rust-analyzer" ];
        };

        # Loaded at runtime by winit / glow / wgpu
        runtimeLibs = with pkgs; [
          # keyboard
          libxkbcommon
          # Wayland
          wayland
          # X11 (fallback, and XWayland)
          libX11
          libXcursor
          libXi
          libXrandr
          # rendering: libGL for the default glow backend, vulkan for wgpu
          libGL
          vulkan-loader
        ];
      in {
        devShells.default = pkgs.mkShell {
          nativeBuildInputs = [ rust pkgs.pkg-config ];
          buildInputs = [ pkgs.alsa-lib ] ++ runtimeLibs;

          LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath runtimeLibs;
        };
      });
}

{
  description = "A Nix Flake for Tempest Forge (Bevy 0.18)";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, utils }:
    utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs { inherit system; };

        # Native dependencies needed by Bevy at runtime/compile-time
        bevyDeps = with pkgs; [
          udev
          alsa-lib
          vulkan-loader
          libxkbcommon
          wayland
          libX11
          libXcursor
          libXi
          libXrandr
          libGL
          llvmPackages_18.libllvm
          libdrm
          libxshmfence
          elfutils
          libxcb
          libXext
          libXfixes
          zstd
          zlib
          expat
          pkgs."libxcb-keysyms"
          pkgs."libxcb-image"
          pkgs."libxcb-render-util"
          pkgs."libxcb-wm"
          xcbutil
          stdenv.cc.cc.lib
        ];
      in
      {
        devShells.default = pkgs.mkShell {
          name = "tempest-forge-dev";

          nativeBuildInputs = with pkgs; [
            pkg-config
            # Include rustup if needed, or use the host toolchain
            rustup
          ];

          buildInputs = bevyDeps;

          shellHook = ''
            mkdir -p .nix-libs
            ln -sf ${pkgs.spirv-tools.lib}/lib/libSPIRV-Tools-shared.so .nix-libs/libSPIRV-Tools.so
            export LD_LIBRARY_PATH="$LD_LIBRARY_PATH:${pkgs.lib.makeLibraryPath bevyDeps}:$PWD/.nix-libs"
            echo "===================================================="
            echo "           Tempest Forge Dev Shell Activated        "
            echo "   All native system dependencies for Bevy 0.18      "
            echo "   are loaded. Run 'cargo run' or 'cargo check'.    "
            echo "===================================================="
          '';
        };
      }
    );
}

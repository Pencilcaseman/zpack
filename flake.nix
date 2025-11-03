{
  description = "zpack developent enviornment";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = nixpkgs.legacyPackages.${system};
      in
        {
        devShells.default = pkgs.mkShell {
          packages = with pkgs; [
            python3

            # rustc
            # rustup
            # cargo

            clang
            lld
            llvm
            llvmPackages.bintools
            clang-tools
            lldb
          ];

          buildInputs = with pkgs; [
            pkg-config
            zlib
            openssl
          ];
        };
      }
    );
}


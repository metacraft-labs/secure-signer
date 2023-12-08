{
  outputs = inputs @ {
    nixpkgs,
    flake-parts,
    ...
  }:
    flake-parts.lib.mkFlake {inherit inputs;} {
      systems = ["x86_64-linux"];
      perSystem = {pkgs, ...}: {
        devShells.default = with pkgs;
          mkShell {
            packages = [
              cargo
              stdenv.cc
              pkg-config
              openssl.dev
            ];
          };
      };
    };
}

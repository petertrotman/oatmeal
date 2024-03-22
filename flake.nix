{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    flake-utils.url = "github:numtide/flake-utils";
    peternixpkgs.url = "github:petertrotman/nixpkgs/Diagon";
  };

  outputs = { self, nixpkgs, flake-utils, fenix, peternixpkgs }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs { inherit system; overlays = [ fenix.overlays.default ]; };
        peterpkgs = peternixpkgs.legacyPackages."${system}";
      in
      {
        devShell = pkgs.mkShell {
          packages = [
            (pkgs.fenix.fromToolchainFile {
              dir = ./.;
              sha256 = "sha256-U2yfueFohJHjif7anmJB5vZbpP7G6bICH4ZsjtufRoU=";
            })
            pkgs.cargo-run-bin
            peterpkgs.diagon
          ];
        };
      });
}

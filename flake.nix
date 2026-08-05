{
  description = "Native Nix package for the Endfield launcher";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
  };

  outputs = { self, nixpkgs }:
    let
      supportedSystems = [
        "x86_64-linux"
      ];

      forAllSystems = nixpkgs.lib.genAttrs supportedSystems;
    in {
      packages = forAllSystems (system:
        let
          pkgs = import nixpkgs {
            inherit system;
          };

          endfield = pkgs.callPackage ./package.nix {
            src = self;
          };
        in {
          inherit endfield;
          default = endfield;
        });

      apps = forAllSystems (system:
        let
          app = {
            type = "app";
            program = "${self.packages.${system}.endfield}/bin/endfield";
          };
        in {
          endfield = app;
          default = app;
        });
    };
}

{
  description = "Flake for messing";

  inputs = {
    devshell.url = "github:numtide/devshell";
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    naersk = {
      url = "github:nmattia/naersk";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    flakeUtils.url = "github:numtide/flake-utils";
    rustOverlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = inputs: with inputs;
    with flakeUtils.lib;
    eachSystem defaultSystems (system:
      let
        common = import ./nix/common.nix {
          sources = { inherit devshell naersk nixpkgs rustOverlay; };
          inherit system;
        };

        packages = {
          # Compiles slower but has tests and faster executable
          "messing" = import ./nix/build.nix {
            inherit common;
            doCheck = true;
            release = true;
          };
          # Compiles faster but no tests and slower executable
          "messing-debug" = import ./nix/build.nix { inherit common; };
          # Compiles faster but has tests and slower executable
          "messing-tests" = import ./nix/build.nix { inherit common; doCheck = true; };
        };

      in
      {
        inherit packages;

        # Release build is the default package
        defaultPackage = packages."messing";



        devShell = import ./nix/devShell.nix { inherit common; };
      }
    );
}

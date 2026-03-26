{
  description = "A high-performance Wayland screen locker written in Rust";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
  };

  outputs = { self, nixpkgs }: let
    system = builtins.currentSystem;
    pkgs = import nixpkgs { inherit system; };
  in {
    packages.${system}.rustlock = pkgs.callPackage ./default.nix {};
  };
}
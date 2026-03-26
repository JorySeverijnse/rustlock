{
  description = "A high-performance Wayland screen locker written in Rust";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
  };

  outputs = { self, nixpkgs }: let
    system = "x86_64-linux";
    pkgs = import nixpkgs { inherit system; };
  in {
    packages.${system}.default = pkgs.rustPlatform.buildRustPackage {
      pname = "rustlock";
      version = "0.1.0";
      src = ./.;
      cargoLock = { lockFile = ./Cargo.lock; };

      buildInputs = with pkgs; [
        cairo
        pam
        gdk-pixbuf
        librsvg
        pango
        libxkbcommon
        dbus
      ];

      nativeBuildInputs = [
        pkgs.pkg-config
        pkgs.rustPlatform.bindgenHook
        pkgs.rustfmt
        pkgs.clippy
      ];
    };
  };
}
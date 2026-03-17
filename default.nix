{ pkgs ? import <nixpkgs> {} }:

pkgs.rustPlatform.buildRustPackage {
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
  ];
}

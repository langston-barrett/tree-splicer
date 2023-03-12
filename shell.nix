{ pkgs ? import <nixpkgs> { }
, unstable ? import <unstable> { }
}:

pkgs.mkShell {
  nativeBuildInputs = [
    pkgs.rust-analyzer
    pkgs.rustup
  ];
}

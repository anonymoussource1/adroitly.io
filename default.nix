{ nixpkgs ? import <nixpkgs> {  } }:

let
	pkgs = [
		nixpkgs.gcc
		nixpkgs.cargo
		nixpkgs.SDL2
	];
in
	nixpkgs.stdenv.mkDerivation {
		name = "env";
		buildInputs = pkgs;
	}

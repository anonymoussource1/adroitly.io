let
	pkgs = import <nixpkgs> {};
	fenix = import (fetchTarball "https://github.com/nix-community/fenix/archive/main.tar.gz") { };
in
	pkgs.mkShellNoCC {
		packages = with pkgs; [
			gcc
			cargo
			SDL2
			fenix.complete.rustfmt
		];
	}

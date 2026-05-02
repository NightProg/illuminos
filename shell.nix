{ pkgs ? import <nixpkgs> {} }:
pkgs.mkShell {
	packages = with pkgs; [
		nasm
	];
	
	shellHook = ''
		export RUSTUP_PREFIX="${pkgs.rustup}"
	'';
}

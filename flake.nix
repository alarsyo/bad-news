{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    flake-compat = {
      url = "github:edolstra/flake-compat";
      flake = false;
    };
  };

  outputs = { self, nixpkgs, flake-utils, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs { inherit system; };
      in
      {
        defaultPackage = pkgs.rustPlatform.buildRustPackage {
          pname = "bad-news";
          version = "0.1.0";

          src = ./.;

          cargoSha256 = "sha256-dNf/FYhNRu85Q4ZinvFGcJmMRayVdTJ9j28fu9BIinY=";

          meta = with pkgs.lib; {
            description = "A Matrix bot, bringer of bad news";
            homepage = "https://github.com/alarsyo/bad-news";
            license = with licenses; [ mit asl20 ];
            platforms = platforms.unix;
          };

          nativeBuildInputs = with pkgs; [ pkg-config cmake ];
          buildInputs = with pkgs; [ systemd openssl ];
        };

        defaultApp = flake-utils.lib.mkApp {
          drv = self.defaultPackage."${system}";
        };

        devShell = pkgs.mkShell {
          buildInputs = with pkgs; [
            cargo
            clippy
            nixpkgs-fmt
            rustPackages.clippy
            rustc
            rustfmt

            pkg-config
            cmake
            systemd
            openssl
          ];

          RUST_SRC_PATH = pkgs.rustPlatform.rustLibSrc;
        };
      });
}

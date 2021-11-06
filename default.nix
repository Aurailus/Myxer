{ pkgs ? import <nixpkgs> { }, isShell ? false }:

pkgs.rustPlatform.buildRustPackage rec {
  pname = "myxer";
  version = "1.2.1";

  src = ./.;

  cargoLock = { lockFile = ./Cargo.lock; };

  nativeBuildInputs = [ pkgs.pkg-config ];

  buildInputs = with pkgs;
    if isShell then [
      rust-analyzer
      libpulseaudio
      glib
      pango
      gtk3
    ] else [
      libpulseaudio
      glib
      pango
      gtk3
    ];

  # Currently no tests are implemented, so we avoid building the package twice
  doCheck = false;

  meta = with pkgs.lib; {
    description = "A modern Volume Mixer for PulseAudio";
    homepage = "https://github.com/Aurailus/Myxer";
    license = licenses.gpl3Only;
    maintainers = with maintainers; [ erin ];
    platforms = platforms.linux;
  };
}

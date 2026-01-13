{
  pkgs,
  ...
}:

{
  packages = with pkgs; [
    hexedit
    wxhexeditor
    cargo-watch
  ];

  languages.rust.enable = true;
}

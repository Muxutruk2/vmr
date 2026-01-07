{
  pkgs,
  ...
}:

{
  packages = with pkgs; [ hexedit ];

  languages.rust.enable = true;
}

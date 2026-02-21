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

  git-hooks.hooks = {
    clippy = {
      enable = true;
      settings = {
        allFeatures = true;
        denyWarnings = true;
      };
    };
    rustfmt = {
      enable = true;
    };
  };
}

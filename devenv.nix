{
  pkgs,
  config,
  ...
}:

{
  env."VMR_ASSEMBLER" = "${config.env.DEVENV_ROOT}/target/release/assembler";
  env."VMR_RUNNER" = "${config.env.DEVENV_ROOT}/target/release/runner";

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

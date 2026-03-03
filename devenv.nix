{
  pkgs,
  config,
  ...
}:

{
  env."VMR_LINKER" = "${config.env.DEVENV_ROOT}/target/release/linker";
  env."VMR_ASSEMBLER" = "${config.env.DEVENV_ROOT}/target/release/assembler";
  env."VMR_RUNNER" = "${config.env.DEVENV_ROOT}/target/release/runner";

  env."RUST_LOG" = "info";

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

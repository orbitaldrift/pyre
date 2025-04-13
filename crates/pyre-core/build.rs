fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    pyre_build::emit_build_info()
}

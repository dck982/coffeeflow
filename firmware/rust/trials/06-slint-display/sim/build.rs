fn main() {
    // Même .slint et même pré-rendu des glyphes que sur la carte.
    slint_build::compile_with_config(
        "../ui/main.slint",
        slint_build::CompilerConfiguration::new()
            .embed_resources(slint_build::EmbedResourcesKind::EmbedForSoftwareRenderer),
    )
    .unwrap();
}

fn main() {
    // Les tailles d'une instance de composant Text ne sont pas toutes
    // détectées par l'embarquement des glyphes ; les annoncer évite qu'un
    // héros de 76 px soit dessiné avec les seuls glyphes de 26 px.
    // Le build n'a encore lancé aucun thread lorsque l'environnement change.
    unsafe { std::env::set_var("SLINT_FONT_SIZES", "16,18,26,28,32,40,76,104") };
    slint_build::compile_with_config(
        "../screen/ui/app.slint",
        slint_build::CompilerConfiguration::new()
            .embed_resources(slint_build::EmbedResourcesKind::EmbedForSoftwareRenderer),
    )
    .unwrap();
}

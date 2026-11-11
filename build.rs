fn main() {
    // Bundle the custom toolbar icons from src/assets/icons into a GResource.
    relm4_icons_build::bundle_icons(
        "icon_names.rs",
        Some("rocks.searching.Rocks"),
        None,
        Some("src/assets/icons"),
        std::iter::empty::<&str>(),
    );
}

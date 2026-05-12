use ladder_core::slug::slugify;

#[test]
fn slugify_lowercases_and_hyphenates_upstream_names() {
    assert_eq!(slugify("Aither"), "aither");
    assert_eq!(slugify("OnlyEncodes+"), "onlyencodes");
    assert_eq!(slugify("U2.dmhy"), "u2-dmhy");
    assert_eq!(slugify("White Rabbit Network"), "white-rabbit-network");
    assert_eq!(slugify("Cathode-Ray.Tube"), "cathode-ray-tube");
}

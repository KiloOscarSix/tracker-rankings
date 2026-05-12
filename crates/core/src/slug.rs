/// Lowercase ASCII alphanumerics joined by single hyphens, e.g.
/// `"U2.dmhy"` becomes `"u2-dmhy"`.
#[must_use]
pub fn slugify(name: &str) -> String {
    let mut slug = String::with_capacity(name.len());
    for word in
        name.split(|c: char| !c.is_ascii_alphanumeric()).filter(|w| !w.is_empty())
    {
        if !slug.is_empty() {
            slug.push('-');
        }
        slug.push_str(&word.to_ascii_lowercase());
    }
    slug
}

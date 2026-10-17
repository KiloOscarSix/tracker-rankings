use std::collections::BTreeSet;

use rankings_core::category::Category;
use rankings_core::error::{UnknownCategoryKey, UnknownCategoryToken};

fn upstream_tokens() -> Option<BTreeSet<String>> {
    let value = serde_json::from_str::<serde_json::Value>(include_str!(
        "../../../data/trackerpathways/trackers.json"
    ))
    .ok()?;
    let mut tokens = BTreeSet::new();
    for info in value.get("trackerInfo")?.as_object()?.values() {
        let raw_type = info.get("type")?.as_str()?;
        tokens.extend(
            raw_type
                .split(',')
                .map(str::trim)
                .filter(|t| !t.is_empty())
                .map(str::to_owned),
        );
    }
    Some(tokens)
}

#[test]
fn every_upstream_type_token_maps_to_a_category() {
    let tokens = upstream_tokens().unwrap();
    assert_eq!(tokens.len(), 67);
    for token in &tokens {
        assert!(
            Category::from_token(token).is_ok(),
            "upstream type token {token:?} has no category"
        );
    }
}

#[test]
fn upstream_tokens_map_to_the_pinned_categories() {
    const EXPECTED: &[(&str, Category)] = &[
        ("Movie", Category::MoviesTv),
        ("Movies", Category::MoviesTv),
        ("TV", Category::MoviesTv),
        ("Tv", Category::MoviesTv),
        ("Animated", Category::MoviesTv),
        ("Cartoon", Category::MoviesTv),
        ("Classic", Category::MoviesTv),
        ("Vintage", Category::MoviesTv),
        ("Old/Obscure", Category::MoviesTv),
        ("Arthouse/Obscure", Category::MoviesTv),
        ("Discs Only", Category::MoviesTv),
        ("FANRES", Category::MoviesTv),
        ("Documentaries", Category::MoviesTv),
        ("Comedy", Category::MoviesTv),
        ("Theatre", Category::MoviesTv),
        ("Sports", Category::MoviesTv),
        ("Motorsports", Category::MoviesTv),
        ("Racing", Category::MoviesTv),
        ("Music", Category::Music),
        ("Audio", Category::Music),
        ("Music Production", Category::Music),
        ("Podcasts", Category::Music),
        ("Books", Category::Books),
        ("eBooks", Category::Books),
        ("Audiobooks", Category::Books),
        ("Comics", Category::Books),
        ("Magazines", Category::Books),
        ("Anime", Category::Anime),
        ("Manga", Category::Anime),
        ("Games", Category::GamesSoftware),
        ("PC", Category::GamesSoftware),
        ("Console", Category::GamesSoftware),
        ("Mac Software", Category::GamesSoftware),
        ("0DAY", Category::GamesSoftware),
        ("3d Printing", Category::GamesSoftware),
        ("CG", Category::GamesSoftware),
        ("VFX", Category::GamesSoftware),
        ("General", Category::General),
        ("Scene", Category::General),
        ("Requests", Category::General),
        ("E-Learning", Category::ELearning),
        ("Business", Category::ELearning),
        ("Self-help", Category::ELearning),
        ("Pick-up", Category::ELearning),
        ("Premium Videos", Category::ELearning),
        ("Porn", Category::Adult),
        ("XXX", Category::Adult),
        ("JAV", Category::Adult),
        ("Hentai", Category::Adult),
        ("Asian", Category::Regional),
        ("Brazilian", Category::Regional),
        ("Castellano", Category::Regional),
        ("Chinese", Category::Regional),
        ("European", Category::Regional),
        ("French", Category::Regional),
        ("German", Category::Regional),
        ("Hungarian", Category::Regional),
        ("Indian", Category::Regional),
        ("Italian", Category::Regional),
        ("Latino", Category::Regional),
        ("Malaysian", Category::Regional),
        ("Nordic", Category::Regional),
        ("Portuguese", Category::Regional),
        ("Romanian", Category::Regional),
        ("Swedish", Category::Regional),
        ("Turkish", Category::Regional),
        ("Ukrainian", Category::Regional),
    ];

    for &(token, expected) in EXPECTED {
        assert_eq!(Category::from_token(token), Ok(expected), "token {token:?}");
    }
    let table =
        EXPECTED.iter().map(|&(token, _)| token.to_owned()).collect::<BTreeSet<_>>();
    assert_eq!(table, upstream_tokens().unwrap());
}

#[test]
fn normalise_sorts_dedups_and_rejects_unknown_tokens() {
    assert_eq!(
        Category::normalise("Movie, TV, General"),
        Ok(vec![Category::MoviesTv, Category::General])
    );
    assert_eq!(
        Category::normalise("General, Movies, ,TV"),
        Ok(vec![Category::MoviesTv, Category::General])
    );
    assert_eq!(
        Category::normalise("Movie, Knitting"),
        Err(UnknownCategoryToken("Knitting".to_owned()))
    );
}

#[test]
fn keys_round_trip_through_from_str() {
    for category in Category::ALL {
        assert_eq!(category.key().parse::<Category>(), Ok(category));
    }
    assert_eq!(
        "Movies/TV".parse::<Category>(),
        Err(UnknownCategoryKey("Movies/TV".to_owned()))
    );
}

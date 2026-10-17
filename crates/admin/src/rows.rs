use std::collections::{BTreeMap, BTreeSet};

use rankings_core::category::Category;
use rankings_core::slug::slugify;

use crate::error::ImportError;
use crate::seed::Seed;

/// The seed flattened into column vectors for `UNNEST` bulk upserts.
pub(crate) struct SeedRows {
    pub(crate) listed: ListedRows,
    pub(crate) stubs: StubRows,
    pub(crate) routes: RouteRows,
}

#[derive(Default)]
pub(crate) struct ListedRows {
    pub(crate) slugs: Vec<String>,
    pub(crate) names: Vec<String>,
    pub(crate) abbrs: Vec<String>,
    pub(crate) raw_types: Vec<String>,
    /// Category keys joined with ','; see `upsert_listed`.
    pub(crate) categories: Vec<String>,
    pub(crate) descriptions: Vec<String>,
    pub(crate) countries: Vec<Option<String>>,
    pub(crate) founded_years: Vec<Option<i16>>,
    pub(crate) unlock_days: Vec<Option<i32>>,
    pub(crate) unlock_reqs: Vec<Option<String>>,
}

#[derive(Default)]
pub(crate) struct StubRows {
    pub(crate) slugs: Vec<String>,
    pub(crate) names: Vec<String>,
}

#[derive(Default)]
pub(crate) struct RouteRows {
    pub(crate) sources: Vec<String>,
    pub(crate) targets: Vec<String>,
    pub(crate) days: Vec<Option<i32>>,
    pub(crate) reqs: Vec<String>,
    pub(crate) active: Vec<String>,
    pub(crate) updated: Vec<String>,
}

impl SeedRows {
    pub(crate) fn from_seed(seed: &Seed) -> Result<Self, ImportError> {
        if let Some(name) =
            seed.unlocks.keys().find(|n| !seed.trackers.contains_key(*n))
        {
            return Err(ImportError::UnlockWithoutTracker(name.clone()));
        }

        let mut slugs = BTreeMap::new();
        let listed = listed_rows(seed, &mut slugs)?;
        let stubs = stub_rows(seed, &mut slugs)?;
        Ok(Self { listed, stubs, routes: route_rows(seed) })
    }
}

fn listed_rows(
    seed: &Seed,
    slugs: &mut BTreeMap<String, String>,
) -> Result<ListedRows, ImportError> {
    let mut rows = ListedRows::default();
    for (name, info) in &seed.trackers {
        let unlock = seed.unlocks.get(name);
        rows.slugs.push(claim_slug(slugs, name)?);
        rows.names.push(name.clone());
        rows.abbrs.push(info.abbr.clone());
        rows.raw_types.push(info.raw_type.clone());
        let categories = Category::normalise(&info.raw_type).map_err(|source| {
            ImportError::Category { tracker: name.clone(), source }
        })?;
        rows.categories
            .push(categories.iter().map(|c| c.key()).collect::<Vec<_>>().join(","));
        rows.descriptions.push(info.description.clone());
        rows.countries.push(non_empty(&info.country));
        rows.founded_years.push(info.birthdate.year(name)?);
        rows.unlock_days.push(unlock.and_then(|u| u.0));
        rows.unlock_reqs.push(unlock.and_then(|u| non_empty(&u.1)));
    }
    Ok(rows)
}

fn stub_rows(
    seed: &Seed,
    slugs: &mut BTreeMap<String, String>,
) -> Result<StubRows, ImportError> {
    let endpoints: BTreeSet<&String> = seed
        .routes
        .iter()
        .flat_map(|(source, targets)| std::iter::once(source).chain(targets.keys()))
        .filter(|name| !seed.trackers.contains_key(*name))
        .collect();

    let mut rows = StubRows::default();
    for name in endpoints {
        rows.slugs.push(claim_slug(slugs, name)?);
        rows.names.push(name.clone());
    }
    Ok(rows)
}

fn route_rows(seed: &Seed) -> RouteRows {
    let mut rows = RouteRows::default();
    for (source, targets) in &seed.routes {
        for (target, route) in targets {
            rows.sources.push(source.clone());
            rows.targets.push(target.clone());
            rows.days.push(route.days);
            rows.reqs.push(route.reqs.clone());
            rows.active.push(route.active.clone());
            rows.updated.push(route.updated.clone());
        }
    }
    rows
}

fn claim_slug(
    slugs: &mut BTreeMap<String, String>,
    name: &str,
) -> Result<String, ImportError> {
    let slug = slugify(name);
    if slug.is_empty() {
        return Err(ImportError::EmptySlug(name.to_owned()));
    }
    if let Some(first) = slugs.insert(slug.clone(), name.to_owned()) {
        return Err(ImportError::SlugCollision {
            slug,
            first,
            second: name.to_owned(),
        });
    }
    Ok(slug)
}

fn non_empty(text: &str) -> Option<String> {
    (!text.is_empty()).then(|| text.to_owned())
}

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use anyhow::{Context, bail};
use entl_codebase::{DependencySource, InventoryOptions};
use entl_semantics::SemanticObservations;
use entl_tree_sitter::ParserCatalog;
use infact_analysis::{AnalysisSelection, FactBatch, FactPackSet};
use infact_fact_builder::{ExternalFactPackBuilder, FactPackBuildRequest};
use infact_fact_pack::{CachedFactPack, FactPackCache, FactPackLock};
use infact_fact_registry::{FactPackRegistry, FactPackRegistryAuth};

use crate::config::{DependencySelection, FactBuilder, FactSettings};

pub struct FactRuntime {
    parsers: ParserCatalog,
    packs: FactPackSet,
    observations: Vec<SemanticObservations>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FactStatus {
    pub name: String,
    pub version: String,
    pub revision: u32,
    pub digest: String,
    pub origin: Option<String>,
    pub cached: bool,
}

#[derive(Debug)]
pub struct FactSync {
    pub packs: Vec<CachedFactPack>,
    pub unavailable: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct FactRequest {
    ecosystem: String,
    name: String,
    version: String,
    required: bool,
}

const LIBRARY_BEHAVIOR_CAPABILITY: &str = "library-behaviors";

/// Report the packages whose locked fact packs describe library behaviors.
///
/// Dependencies are the configuration. A synchronized lock records a pack for
/// every dependency Infact can describe, so this is the set of libraries the
/// repository already depends on and Straitjacket can hold it to.
pub fn library_behavior_packages(settings: &FactSettings) -> BTreeSet<String> {
    let Ok(lock) = FactPackLock::read_or_default(&settings.lock) else {
        return BTreeSet::new();
    };
    lock.packs
        .into_iter()
        .filter(|pack| {
            pack.manifest
                .provides
                .iter()
                .any(|capability| capability.ends_with(LIBRARY_BEHAVIOR_CAPABILITY))
        })
        .map(|pack| pack.manifest.subject.name)
        .collect()
}

impl FactRuntime {
    pub fn load(settings: &FactSettings, selection: &AnalysisSelection) -> anyhow::Result<Self> {
        if settings.parser_paths.is_empty() {
            bail!("fact-backed rules require [facts].parser-paths");
        }
        let discovery = ParserCatalog::discover(settings.parser_paths.clone());
        if !discovery.errors.is_empty() {
            bail!(
                "loading Entl parser packs: {}",
                discovery
                    .errors
                    .into_iter()
                    .map(|error| error.to_string())
                    .collect::<Vec<_>>()
                    .join("; ")
            );
        }
        let locked = if selection.library_behaviors || selection.call_effects {
            let lock = FactPackLock::read(&settings.lock).with_context(|| {
                format!(
                    "loading fact lock {}; run `straitjacket facts sync`",
                    settings.lock.display()
                )
            })?;
            let cache = FactPackCache::open(&settings.cache)?;
            lock.verify(&cache)?
        } else {
            Vec::new()
        };
        let packs = FactPackSet::load(&locked)?;
        if selection.call_effects && packs.call_effects().is_empty() {
            bail!(
                "effect capability checks require a locked call-effects fact pack; run `straitjacket facts sync`"
            );
        }
        packs.validate_runtime(&discovery.catalog)?;
        Ok(Self {
            parsers: discovery.catalog,
            packs,
            observations: load_observations(settings.observations.as_deref())?,
        })
    }

    pub fn analyze(&self, root: &Path, selection: &AnalysisSelection) -> anyhow::Result<FactBatch> {
        // A provider recorded paths from wherever the build ran; a scan may be
        // rooted at a subdirectory, so the two have to be related before the
        // observations mean anything here.
        let observations = self
            .observations
            .iter()
            .cloned()
            .map(|mut unit| {
                unit.rebase(root);
                unit
            })
            .collect::<Vec<_>>();
        infact_analysis::analyze_repository_with_observations(
            root,
            &self.parsers,
            &self.packs,
            selection,
            &observations,
        )
        .map_err(Into::into)
    }
}

pub fn status(settings: &FactSettings) -> anyhow::Result<Vec<FactStatus>> {
    let lock = FactPackLock::read(&settings.lock)?;
    let cache = settings
        .cache
        .exists()
        .then(|| FactPackCache::open(&settings.cache))
        .transpose()?;
    lock.packs
        .into_iter()
        .map(|entry| {
            let cached = cache
                .as_ref()
                .is_some_and(|cache| cache.load(&entry.manifest_digest).is_ok());
            Ok(FactStatus {
                name: entry.manifest.name,
                version: entry.manifest.subject.version,
                revision: entry.manifest.revision,
                digest: entry.manifest_digest,
                origin: entry.origin,
                cached,
            })
        })
        .collect()
}

pub async fn sync(
    settings: &FactSettings,
    offline: bool,
    prebuilt_only: bool,
) -> anyhow::Result<FactSync> {
    validate_builders(settings)?;
    let cache = FactPackCache::open(&settings.cache)?;
    if offline {
        return Ok(FactSync {
            packs: FactPackLock::read(&settings.lock)?.verify(&cache)?,
            unavailable: Vec::new(),
        });
    }

    let previous = FactPackLock::read_or_default(&settings.lock)?;
    let mut next = FactPackLock::default();
    let registry = FactPackRegistry::default();
    let auth = FactPackRegistryAuth::Anonymous;
    let mut resolved = Vec::new();
    let mut unavailable = Vec::new();
    for request in requests(settings)? {
        let ecosystem = request.ecosystem.as_str();
        let name = request.name.as_str();
        let requested_version = request.version.as_str();
        let description = format!("{ecosystem}:{name}@{requested_version}");
        if let Some(pack) = previous
            .packs
            .iter()
            .find(|entry| {
                entry.manifest.subject.ecosystem.as_deref() == Some(ecosystem)
                    && entry.manifest.subject.name == name
                    && version_matches(&entry.manifest.subject.version, requested_version)
            })
            .and_then(|entry| cache.load(&entry.manifest_digest).ok())
        {
            let origin = previous
                .packs
                .iter()
                .find(|entry| entry.manifest_digest == pack.manifest_digest)
                .and_then(|entry| entry.origin.clone());
            next.insert(&pack, origin)?;
            resolved.push(pack);
            continue;
        }

        let pack_name = pack_name(ecosystem, name)?;
        let mut failures = Vec::new();
        let mut found = None;
        for base in &settings.registries {
            let repository = format!("{}/{pack_name}:latest", base.trim_end_matches('/'));
            let tags = match registry.list_tags(&repository, &auth).await {
                Ok(tags) => tags,
                Err(error) => {
                    failures.push(error.to_string());
                    continue;
                }
            };
            for tag in candidate_tags(&tags, requested_version) {
                let reference = format!("{}/{}:{tag}", base.trim_end_matches('/'), pack_name);
                match registry.pull(&reference, &auth, None, &cache).await {
                    Ok(pack) if pack_satisfies(&pack, &request) => {
                        found = Some((pack, reference));
                        break;
                    }
                    Ok(_) => failures.push(format!("{reference} does not describe {description}")),
                    Err(error) => failures.push(error.to_string()),
                }
            }
            if found.is_some() {
                break;
            }
        }
        if found.is_none() && settings.build_missing && !prebuilt_only {
            let builder = settings
                .builders
                .iter()
                .find(|builder| builder.ecosystem == request.ecosystem)
                .ok_or_else(|| {
                    anyhow::anyhow!(
                        "no prebuilt fact pack satisfies {description}, and no local builder is configured for {ecosystem}"
                    )
                })?;
            let pack = run_builder(settings, builder, &request, &cache)?;
            found = Some((pack, format!("local-builder:{}", builder.command.join(" "))));
        }
        let Some((pack, origin)) = found else {
            if request.required {
                bail!(
                    "no prebuilt fact pack satisfies {description}: {}",
                    failures.join("; ")
                );
            }
            unavailable.push(description);
            continue;
        };
        next.insert(&pack, Some(origin))?;
        resolved.push(pack);
    }
    next.write(&settings.lock)?;
    Ok(FactSync {
        packs: resolved,
        unavailable,
    })
}

fn requests(settings: &FactSettings) -> anyhow::Result<Vec<FactRequest>> {
    let mut requests = BTreeMap::<(String, String, String), bool>::new();
    if settings.dependencies == DependencySelection::Automatic {
        for request in automatic_dependencies(settings)? {
            requests.insert(
                (request.ecosystem, request.name, request.version),
                request.required,
            );
        }
    }
    Ok(requests
        .into_iter()
        .map(|((ecosystem, name, version), required)| FactRequest {
            ecosystem,
            name,
            version,
            required,
        })
        .collect())
}

fn automatic_dependencies(settings: &FactSettings) -> anyhow::Result<Vec<FactRequest>> {
    let inventory = entl_codebase::inspect(&settings.repository_root, &InventoryOptions::default())
        .with_context(|| {
            format!(
                "discovering dependencies in {}",
                settings.repository_root.display()
            )
        })?;
    let mut requests = BTreeSet::new();
    if settings.require_call_effects
        && inventory
            .packages
            .iter()
            .any(|package| package.kind == entl_codebase::PackageKind::Cargo)
    {
        let compiler = entl_codebase::observe_rust_compiler(&settings.repository_root)
            .context("observing the active Rust compiler")?;
        requests.insert(FactRequest {
            ecosystem: "cargo".to_owned(),
            name: "core".to_owned(),
            version: compiler.version,
            required: true,
        });
    }
    for package in &inventory.packages {
        let Some(lockfile) = &package.lockfile else {
            continue;
        };
        let Some(resolution) = inventory
            .dependency_resolutions
            .iter()
            .find(|resolution| resolution.lockfile == *lockfile)
        else {
            continue;
        };
        for dependency in &package.dependencies {
            if dependency.source == DependencySource::Unknown {
                continue;
            }
            for resolved in resolution
                .packages
                .iter()
                .filter(|resolved| resolved.name == dependency.package_name())
            {
                requests.insert(FactRequest {
                    ecosystem: resolution.ecosystem.to_string(),
                    name: resolved.name.clone(),
                    version: resolved.version.clone(),
                    required: settings.require_call_effects,
                });
            }
        }
    }
    Ok(requests.into_iter().collect())
}

fn validate_builders(settings: &FactSettings) -> anyhow::Result<()> {
    let mut ecosystems = BTreeSet::new();
    for builder in &settings.builders {
        if builder.ecosystem.is_empty() {
            bail!("fact builder ecosystem cannot be empty");
        }
        if builder.command.is_empty() {
            bail!(
                "fact builder for ecosystem {} has an empty command",
                builder.ecosystem
            );
        }
        if !ecosystems.insert(&builder.ecosystem) {
            bail!(
                "multiple fact builders are configured for ecosystem {}",
                builder.ecosystem
            );
        }
    }
    Ok(())
}

fn run_builder(
    settings: &FactSettings,
    builder: &FactBuilder,
    request: &FactRequest,
    cache: &FactPackCache,
) -> anyhow::Result<CachedFactPack> {
    let builder = ExternalFactPackBuilder::new(builder.command.clone())?;
    let pack = builder.build(
        &FactPackBuildRequest {
            ecosystem: &request.ecosystem,
            package: &request.name,
            version: &request.version,
            repository: &settings.repository_root,
        },
        cache,
    )?;
    if !pack_satisfies(&pack, request) {
        bail!(
            "local fact builder produced {}:{}@{}, expected {}:{}@{}",
            pack.manifest
                .subject
                .ecosystem
                .as_deref()
                .unwrap_or("<none>"),
            pack.manifest.subject.name,
            pack.manifest.subject.version,
            request.ecosystem,
            request.name,
            request.version
        );
    }
    Ok(pack)
}

fn pack_satisfies(pack: &CachedFactPack, request: &FactRequest) -> bool {
    pack.manifest.subject.ecosystem.as_deref() == Some(request.ecosystem.as_str())
        && pack.manifest.subject.name == request.name
        && version_matches(&pack.manifest.subject.version, &request.version)
}

fn pack_name(ecosystem: &str, name: &str) -> anyhow::Result<String> {
    match ecosystem {
        "cargo" => Ok(format!("rust-{name}")),
        _ => bail!("fact-pack naming is not defined for ecosystem {ecosystem}"),
    }
}

fn candidate_tags(tags: &[String], requested_version: &str) -> Vec<String> {
    let mut candidates = tags
        .iter()
        .filter_map(|tag| {
            let (tag_version, revision) = tag.rsplit_once("-r")?;
            let revision = revision.parse::<u32>().ok()?;
            let version = decode_tag_version(tag_version);
            let parsed = semver::Version::parse(&version).ok()?;
            version_matches(&version, requested_version).then(|| (parsed, revision, tag.clone()))
        })
        .collect::<Vec<_>>();
    candidates.sort_by(|left, right| right.cmp(left));
    candidates.into_iter().map(|(_, _, tag)| tag).collect()
}

fn decode_tag_version(version: &str) -> String {
    version.replacen('_', "+", 1)
}

fn version_matches(actual: &str, requested: &str) -> bool {
    actual == requested
        || actual
            .strip_prefix(requested)
            .is_some_and(|suffix| suffix.starts_with('.'))
}

/// Read every observation file a provider left in a directory.
///
/// A provider writes one file per compilation unit. Missing observations are
/// not an error: analysis falls back to syntax, which is the floor.
fn load_observations(directory: Option<&Path>) -> anyhow::Result<Vec<SemanticObservations>> {
    let Some(directory) = directory else {
        return Ok(Vec::new());
    };
    if !directory.is_dir() {
        return Ok(Vec::new());
    }
    let mut entries = std::fs::read_dir(directory)
        .with_context(|| format!("reading observations in {}", directory.display()))?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.extension()
                .is_some_and(|extension| extension == "json")
        })
        .collect::<Vec<_>>();
    entries.sort();
    entries
        .into_iter()
        .map(|path| {
            let source = std::fs::read(&path)
                .with_context(|| format!("reading observations {}", path.display()))?;
            serde_json::from_slice(&source)
                .with_context(|| format!("parsing observations {}", path.display()))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use std::fs;

    use crate::config::Settings;

    use super::{candidate_tags, requests, version_matches};

    #[test]
    fn requested_versions_accept_a_more_specific_pack_version() {
        assert!(version_matches("0.15.0", "0.15"));
        assert!(version_matches("0.15.0", "0.15.0"));
        assert!(!version_matches("0.150.0", "0.15"));
    }

    #[test]
    fn registry_candidates_prefer_the_highest_revision() {
        assert_eq!(
            candidate_tags(
                &[
                    "0.15.0-r1".to_owned(),
                    "0.15.0-r3".to_owned(),
                    "0.15.1-r1".to_owned(),
                    "0.14.0-r9".to_owned(),
                ],
                "0.15"
            ),
            ["0.15.1-r1", "0.15.0-r3", "0.15.0-r1"]
        );
        assert_eq!(
            candidate_tags(&["1.1.4_spec-1.1.0-r2".to_owned()], "1.1.4+spec-1.1.0"),
            ["1.1.4_spec-1.1.0-r2"]
        );
    }

    #[test]
    fn automatic_dependencies_use_exact_locked_versions() {
        let repository = tempfile::tempdir().unwrap();
        fs::write(
            repository.path().join("Cargo.toml"),
            "[package]\nname='fixture'\nversion='0.0.0'\n[dependencies]\niter={ package='itertools', version='0.15' }\n",
        )
        .unwrap();
        fs::write(
            repository.path().join("Cargo.lock"),
            "version = 4\n\n[[package]]\nname='fixture'\nversion='0.0.0'\ndependencies=['itertools']\n\n[[package]]\nname='itertools'\nversion='0.15.0'\nsource='registry+https://github.com/rust-lang/crates.io-index'\nchecksum='0123456789abcdef'\n",
        )
        .unwrap();
        let mut settings = Settings::default().facts;
        settings.repository_root = repository.path().to_path_buf();

        let without_effects = requests(&settings).unwrap();
        assert!(without_effects.iter().any(|request| {
            request.ecosystem == "cargo"
                && request.name == "itertools"
                && request.version == "0.15.0"
                && !request.required
        }));
        assert!(
            !without_effects.iter().any(|request| request.name == "core"),
            "the language pack is only useful to effect analysis"
        );

        settings.require_call_effects = true;
        let compiler =
            entl_codebase::observe_rust_compiler(repository.path()).expect("active rustc");
        assert!(requests(&settings).unwrap().iter().any(|request| {
            request.ecosystem == "cargo"
                && request.name == "core"
                && request.version == compiler.version
                && request.required
        }));
    }
}

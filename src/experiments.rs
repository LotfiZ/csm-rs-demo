//! Named experiment persistence.
//!
//! Documents are versioned JSON files on the local backend, so they survive
//! browser cleanup and application restarts. Each document holds the source
//! scans, the generation recipe, the full configurations, version information,
//! and a compact snapshot of the observed result. The snapshot is the
//! observation that was saved; a rerun is produced separately so the two are
//! never confused.

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::config::{MatcherConfig, RunRequest};
use crate::import::export_session;
use crate::simulation::SessionRecord;

pub const FORMAT: &str = "csm-rs-workbench-experiment";
pub const VERSION: u32 = 1;
/// Pinned csm-rs revision recorded with every experiment. Update alongside
/// `Cargo.toml` when the library is upgraded.
const LIBRARY_REVISION: &str = "f6adc2d28b28fc567660f64e948095582fd5c9bd";

/// Where experiments live unless `CSM_DEMO_DATA` overrides it.
pub fn default_data_dir() -> PathBuf {
    std::env::var_os("CSM_DEMO_DATA")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("data/experiments"))
}

/// Application, generator, and library versions recorded with an experiment.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Versions {
    pub app: String,
    pub generator: String,
    pub library: String,
}

impl Default for Versions {
    fn default() -> Self {
        Self {
            app: env!("CARGO_PKG_VERSION").to_owned(),
            generator: "1".to_owned(),
            library: LIBRARY_REVISION.to_owned(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExperimentDocument {
    pub format: String,
    pub version: u32,
    pub name: String,
    pub versions: Versions,
    pub session: SessionRecord,
    /// Second configuration, when the experiment was saved from A/B mode.
    #[serde(default)]
    pub matcher_b: Option<MatcherConfig>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SaveExperimentRequest {
    pub name: String,
    pub run: RunRequest,
    #[serde(default)]
    pub matcher_b: Option<MatcherConfig>,
}

#[derive(Serialize, Deserialize)]
pub struct ExperimentList {
    pub names: Vec<String>,
}

/// A loaded or imported experiment plus any version warnings.
#[derive(Serialize, Deserialize)]
pub struct ImportOutcome {
    pub document: ExperimentDocument,
    pub warnings: Vec<String>,
}

/// Validate a portable document and explain any version mismatch.
///
/// An unsupported version is a hard error; a different library or generator
/// version is only a warning, because results are not guaranteed identical.
pub fn inspect(document: ExperimentDocument) -> Result<ImportOutcome, String> {
    if document.format != FORMAT {
        return Err(format!(
            "unsupported experiment format '{}'",
            document.format
        ));
    }
    if document.version != VERSION {
        return Err(format!(
            "unsupported experiment version {}",
            document.version
        ));
    }
    let current = Versions::default();
    let mut warnings = Vec::new();
    if document.versions.library != current.library {
        warnings.push(format!(
            "saved with csm-rs {} but the current library is {}; reruns may differ",
            document.versions.library, current.library
        ));
    }
    if document.versions.generator != current.generator {
        warnings.push(format!(
            "saved by generator {} but the current generator is {}; regenerated scans may differ",
            document.versions.generator, current.generator
        ));
    }
    Ok(ImportOutcome { document, warnings })
}

fn validate_name(name: &str) -> Result<(), String> {
    if name.is_empty() || name.len() > 64 {
        return Err("experiment name must be 1 to 64 characters".to_owned());
    }
    if !name
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        return Err("experiment name may contain only letters, digits, '-' and '_'".to_owned());
    }
    Ok(())
}

fn path_for(dir: &Path, name: &str) -> Result<PathBuf, String> {
    validate_name(name)?;
    Ok(dir.join(format!("{name}.json")))
}

/// Save the current run as a named experiment and return the stored document.
pub fn save(dir: &Path, request: SaveExperimentRequest) -> Result<ExperimentDocument, String> {
    validate_name(&request.name)?;
    let session = export_session(&request.run)?;
    let document = ExperimentDocument {
        format: FORMAT.to_owned(),
        version: VERSION,
        name: request.name.clone(),
        versions: Versions::default(),
        session,
        matcher_b: request.matcher_b,
    };
    fs::create_dir_all(dir).map_err(|error| format!("create data directory: {error}"))?;
    let path = path_for(dir, &request.name)?;
    let json = serde_json::to_string_pretty(&document).map_err(|error| error.to_string())?;
    fs::write(&path, json).map_err(|error| format!("write experiment: {error}"))?;
    Ok(document)
}

/// Load one experiment by name, with version warnings.
pub fn load(dir: &Path, name: &str) -> Result<ImportOutcome, String> {
    let path = path_for(dir, name)?;
    let json = fs::read_to_string(&path).map_err(|_| format!("experiment '{name}' not found"))?;
    let document: ExperimentDocument =
        serde_json::from_str(&json).map_err(|error| error.to_string())?;
    inspect(document)
}

/// List saved experiment names, sorted.
pub fn list(dir: &Path) -> Result<ExperimentList, String> {
    let mut names = Vec::new();
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|ext| ext.to_str()) == Some("json") {
                if let Some(stem) = path.file_stem().and_then(|stem| stem.to_str()) {
                    names.push(stem.to_owned());
                }
            }
        }
    }
    names.sort();
    Ok(ExperimentList { names })
}

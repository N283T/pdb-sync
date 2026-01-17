use crate::error::{PdbCliError, Result};
use crate::files::PdbId;
use chrono::{DateTime, FixedOffset, NaiveDate};
use serde::{Deserialize, Serialize};

const RCSB_API_BASE: &str = "https://data.rcsb.org/rest/v1/core/entry";

/// Client for interacting with RCSB Data API
pub struct RcsbClient {
    client: reqwest::Client,
}

impl RcsbClient {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }

    /// Fetch entry metadata from RCSB
    pub async fn fetch_entry(&self, pdb_id: &PdbId) -> Result<EntryMetadata> {
        let url = format!("{}/{}", RCSB_API_BASE, pdb_id.as_str().to_uppercase());

        let response = self.client.get(&url).send().await?;

        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Err(PdbCliError::Download(format!(
                "PDB entry {} not found",
                pdb_id
            )));
        }

        if !response.status().is_success() {
            return Err(PdbCliError::Download(format!(
                "API request failed with status {}",
                response.status()
            )));
        }

        let entry: EntryMetadata = response.json().await?;
        Ok(entry)
    }
}

impl Default for RcsbClient {
    fn default() -> Self {
        Self::new()
    }
}

/// Entry metadata from RCSB Data API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntryMetadata {
    /// Entry ID
    pub rcsb_id: String,

    /// Structural information (title, etc.)
    #[serde(rename = "struct")]
    pub struct_info: Option<StructInfo>,

    /// Accession information (dates)
    pub rcsb_accession_info: Option<AccessionInfo>,

    /// Experimental method information
    #[serde(default)]
    pub exptl: Vec<ExperimentalMethod>,

    /// Refinement information (resolution)
    #[serde(default)]
    pub refine: Vec<Refinement>,

    /// Entry information
    pub rcsb_entry_info: Option<EntryInfo>,

    /// Entry container identifiers
    pub rcsb_entry_container_identifiers: Option<ContainerIdentifiers>,
}

/// Structural information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StructInfo {
    /// Title of the entry
    pub title: Option<String>,
}

/// Accession information with dates
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessionInfo {
    /// Deposition date
    pub deposit_date: Option<DateTime<FixedOffset>>,

    /// Initial release date
    pub initial_release_date: Option<DateTime<FixedOffset>>,

    /// Revision date
    pub revision_date: Option<DateTime<FixedOffset>>,
}

/// Experimental method
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExperimentalMethod {
    /// Method name (e.g., "X-RAY DIFFRACTION")
    pub method: Option<String>,
}

/// Refinement information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Refinement {
    /// Resolution in angstroms
    #[serde(rename = "ls_dres_high")]
    pub resolution: Option<f64>,
}

/// Entry information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntryInfo {
    /// Number of polymer entities
    pub polymer_entity_count: Option<u32>,

    /// Number of assemblies
    pub assembly_count: Option<u32>,

    /// Molecular weight in Da
    pub molecular_weight: Option<f64>,
}

/// Container identifiers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerIdentifiers {
    /// Entry ID
    pub entry_id: Option<String>,
}

impl EntryMetadata {
    /// Get the title
    pub fn title(&self) -> Option<&str> {
        self.struct_info.as_ref().and_then(|s| s.title.as_deref())
    }

    /// Get the deposition date
    pub fn deposit_date(&self) -> Option<NaiveDate> {
        self.rcsb_accession_info
            .as_ref()
            .and_then(|a| a.deposit_date.map(|dt| dt.date_naive()))
    }

    /// Get the release date
    pub fn release_date(&self) -> Option<NaiveDate> {
        self.rcsb_accession_info
            .as_ref()
            .and_then(|a| a.initial_release_date.map(|dt| dt.date_naive()))
    }

    /// Get the revision date
    pub fn revision_date(&self) -> Option<NaiveDate> {
        self.rcsb_accession_info
            .as_ref()
            .and_then(|a| a.revision_date.map(|dt| dt.date_naive()))
    }

    /// Get the experimental method
    pub fn method(&self) -> Option<&str> {
        self.exptl.first().and_then(|e| e.method.as_deref())
    }

    /// Get the resolution in angstroms
    pub fn resolution(&self) -> Option<f64> {
        self.refine.first().and_then(|r| r.resolution)
    }

    /// Get the number of polymer entities
    pub fn polymer_entity_count(&self) -> Option<u32> {
        self.rcsb_entry_info
            .as_ref()
            .and_then(|i| i.polymer_entity_count)
    }

    /// Get the number of assemblies
    pub fn assembly_count(&self) -> Option<u32> {
        self.rcsb_entry_info.as_ref().and_then(|i| i.assembly_count)
    }

    /// Get the molecular weight in Da
    pub fn molecular_weight(&self) -> Option<f64> {
        self.rcsb_entry_info
            .as_ref()
            .and_then(|i| i.molecular_weight)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_fetch_entry_4hhb() {
        let client = RcsbClient::new();
        let pdb_id = PdbId::new("4hhb").unwrap();
        let result = client.fetch_entry(&pdb_id).await;

        // This test requires network access
        if let Ok(entry) = result {
            assert_eq!(entry.rcsb_id.to_lowercase(), "4hhb");
            assert!(entry.title().is_some());
        }
    }

    #[tokio::test]
    async fn test_fetch_entry_not_found() {
        let client = RcsbClient::new();
        let pdb_id = PdbId::new("9zzz").unwrap();
        let result = client.fetch_entry(&pdb_id).await;

        // Should return an error for non-existent entry
        assert!(result.is_err());
    }
}

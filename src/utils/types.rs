use crate::alerts::spc::drift::SpcDrifter;
use crate::alerts::types::Drifter;
use scouter::core::drift::base::DriftType;
use scouter::core::drift::spc::types::SpcDriftProfile;

pub enum DriftProfile {
    SpcDriftProfile(SpcDriftProfile),
}

impl DriftProfile {
    pub fn from_type(drift_type: DriftType, profile: String) -> Result<Self, anyhow::Error> {
        match drift_type {
            DriftType::SPC => {
                let profile = serde_json::from_str(&profile)?;
                Ok(DriftProfile::SpcDriftProfile(profile))
            }
            DriftType::PSI => todo!(),
            DriftType::NONE => todo!(),
        }
    }
    pub fn get_drifter(&self, name: String, repository: String, version: String) -> Drifter {
        match self {
            DriftProfile::SpcDriftProfile(profile) => {
                Drifter::SpcDrifter(SpcDrifter::new(name, repository, version, profile.clone()))
            }
        }
    }

    pub fn profile_type(&self) -> String {
        match self {
            DriftProfile::SpcDriftProfile(_) => DriftType::SPC.value(),
        }
    }
}

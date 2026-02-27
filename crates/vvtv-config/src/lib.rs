use std::{fs, path::Path, sync::Arc};

use anyhow::{Context, Result, anyhow};
use parking_lot::RwLock;
use vvtv_types::OwnerCard;

#[derive(Clone)]
pub struct OwnerCardStore {
    path: String,
    current: Arc<RwLock<OwnerCard>>,
}

impl OwnerCardStore {
    pub fn load_from_path(path: impl AsRef<Path>) -> Result<Self> {
        let path_ref = path.as_ref();
        let parsed = load_owner_card(path_ref)?;
        Ok(Self {
            path: path_ref.to_string_lossy().to_string(),
            current: Arc::new(RwLock::new(parsed)),
        })
    }

    pub fn current(&self) -> OwnerCard {
        self.current.read().clone()
    }

    pub fn reload(&self) -> Result<()> {
        let updated = load_owner_card(&self.path)?;
        *self.current.write() = updated;
        Ok(())
    }
}

pub fn load_owner_card(path: impl AsRef<Path>) -> Result<OwnerCard> {
    let raw = fs::read_to_string(path.as_ref())
        .with_context(|| format!("failed reading owner card at {}", path.as_ref().display()))?;
    let card: OwnerCard = serde_yaml::from_str(&raw).context("failed parsing owner card YAML")?;
    card.validate().map_err(|err| anyhow!(err))?;
    Ok(card)
}

use anyhow::Result;
use derive_config::DeriveTomlConfig;
use inquire::{
    list_option::ListOption,
    validator::{ErrorMessage, Validation},
    Confirm,
    MultiSelect,
    Select,
};
use serde::{Deserialize, Serialize};
use strum::{Display, IntoEnumIterator};

use crate::{discord, discord::DEVELOPER_ID, provider::ProviderKind};

#[derive(Display, Deserialize, Serialize)]
pub enum Attribution {
    #[strum(to_string = "Anonymously (VRC-LOG Dev)")]
    Anonymous(String),
    #[strum(to_string = "Discord RPC ({0})")]
    DiscordRPC(String),
    #[strum(to_string = "Discord ID (Manual Input)")]
    DiscordID(String),
}

impl Attribution {
    #[must_use]
    pub fn get_user_id(&self) -> String {
        match self {
            Self::Anonymous(_) => DEVELOPER_ID.to_string(),
            Self::DiscordID(id) => id.clone(),
            Self::DiscordRPC(id) => discord::get_user()
                .and_then(|u| u.id)
                .unwrap_or_else(|| id.clone()),
        }
    }
}

#[derive(DeriveTomlConfig, Deserialize, Serialize)]
pub struct Settings {
    #[serde(default)]
    pub clear_amplitude: bool,
    pub attribution:     Attribution,
    pub providers:       Vec<ProviderKind>,
}

impl Settings {
    /// # Setup Wizard
    ///
    /// # Errors
    ///
    /// Will return `Err` if prompts fail.
    ///
    /// # Panics
    ///
    /// Will panic if Discord user ID doesn't exist.
    pub fn try_wizard() -> Result<Self> {
        let mut attributions = vec![
            Attribution::Anonymous(DEVELOPER_ID.to_string()),
            Attribution::DiscordID(String::new()),
        ];
        if let Some(user) = discord::get_user() {
            attributions.insert(1, Attribution::DiscordRPC(user.id.unwrap()));
        }
        let attribution = Select::new("How do you want to be credited?", attributions).prompt()?;

        let providers = ProviderKind::iter()
            .filter(|provider| !matches!(provider, ProviderKind::CACHE))
            .collect();

        let providers = MultiSelect::new("Select which providers to use:", providers)
            .with_all_selected_by_default()
            .with_validator(|list: &[ListOption<&ProviderKind>]| {
                if list.is_empty() {
                    let message = String::from("You must select at least one.");
                    Ok(Validation::Invalid(ErrorMessage::Custom(message)))
                } else {
                    Ok(Validation::Valid)
                }
            })
            .prompt()?;

        let clear_amplitude = Confirm::new(
            "Clear amplitude file after reading? (Helps with privacy by removing tracked data)",
        )
        .with_default(true)
        .prompt()?;

        Ok(Self {
            clear_amplitude,
            attribution,
            providers,
        })
    }
}

mod avd_list;

use std::{fmt::Display, path::PathBuf};

pub use avd_list::avd_list;

use super::env::Env;
use crate::{bossy, env::ExplicitEnv};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Emulator {
    name: String,
}

impl Display for Emulator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name)
    }
}

impl Emulator {
    fn new(name: String) -> Self {
        Self { name }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn start(&self, env: &Env) -> bossy::Result<bossy::Handle> {
        bossy::Command::impure(PathBuf::from(env.sdk_root()).join("emulator/emulator"))
            .with_args(&["-avd", &self.name])
            .with_env_vars(env.explicit_env())
            .run()
    }
}

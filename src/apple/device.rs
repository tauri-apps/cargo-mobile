use super::{
    config::Config,
    ios_deploy, simctl,
    target::{ArchiveError, BuildError, ExportError, Target},
};
use crate::{
    bossy,
    env::{Env, ExplicitEnv as _},
    opts,
    util::cli::{Report, Reportable},
};
use std::{
    fmt::{self, Display},
    path::PathBuf,
};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum RunError {
    #[error(transparent)]
    BuildFailed(BuildError),
    #[error(transparent)]
    ArchiveFailed(ArchiveError),
    #[error(transparent)]
    ExportFailed(ExportError),
    #[error("IPA appears to be missing. Not found at either {old} or {new}")]
    IpaMissing { old: PathBuf, new: PathBuf },
    #[error("Failed to unzip archive: {0}")]
    UnzipFailed(bossy::Error),
    #[error(transparent)]
    DeployFailed(ios_deploy::RunAndDebugError),
    #[error(transparent)]
    SimulatorDeployFailed(simctl::RunError),
}

impl Reportable for RunError {
    fn report(&self) -> Report {
        match self {
            Self::BuildFailed(err) => err.report(),
            Self::ArchiveFailed(err) => err.report(),
            Self::ExportFailed(err) => err.report(),
            Self::IpaMissing { old, new } => Report::error(
                "IPA appears to be missing",
                format!("Not found at either {:?} or {:?}", old, new),
            ),
            Self::UnzipFailed(err) => Report::error("Failed to unzip archive", err),
            Self::DeployFailed(err) => err.report(),
            Self::SimulatorDeployFailed(err) => err.report(),
        }
    }
}

#[derive(Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct Device<'a> {
    id: String,
    name: String,
    model: String,
    target: &'a Target<'a>,
    simulator: bool,
}

impl<'a> Display for Device<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} ({})", self.name, self.model)
    }
}

impl<'a> Device<'a> {
    pub(super) fn new(id: String, name: String, model: String, target: &'a Target<'a>) -> Self {
        Self {
            id,
            name,
            model,
            target,
            simulator: false,
        }
    }

    pub fn simulator(mut self) -> Self {
        self.simulator = true;
        self
    }

    pub fn target(&self) -> &'a Target<'a> {
        self.target
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn run(
        &self,
        config: &Config,
        env: &Env,
        noise_level: opts::NoiseLevel,
        non_interactive: bool,
        profile: opts::Profile,
    ) -> Result<bossy::Handle, RunError> {
        // TODO: These steps are run unconditionally, which is slooooooow
        println!("Building app...");
        self.target
            .build(config, env, noise_level, profile)
            .map_err(RunError::BuildFailed)?;
        println!("Archiving app...");
        self.target
            .archive(config, env, noise_level, profile, None)
            .map_err(RunError::ArchiveFailed)?;

        if self.simulator {
            simctl::run(config, env, &self.id).map_err(RunError::SimulatorDeployFailed)
        } else {
            println!("Exporting app...");
            self.target
                .export(config, env, noise_level)
                .map_err(RunError::ExportFailed)?;
            println!("Extracting IPA...");
            bossy::Command::pure("unzip")
                .with_env_vars(env.explicit_env())
                .with_args(if noise_level.pedantic() {
                    None
                } else {
                    Some("-q")
                })
                .with_arg("-o") // -o = always overwrite
                .with_arg(
                    &config
                        .ipa_path()
                        .map_err(|(old, new)| RunError::IpaMissing { old, new })?,
                )
                .with_arg("-d")
                .with_arg(&config.export_dir())
                .run_and_wait()
                .map_err(RunError::UnzipFailed)?;

            ios_deploy::run_and_debug(config, env, non_interactive, &self.id)
                .map_err(RunError::DeployFailed)
        }
    }
}

use std::io::Error as IoError;

use fluvio::FluvioError;
use fluvio_cluster::{ClusterError, CheckError, K8InstallError, HelmError};

pub type Result<T> = std::result::Result<T, CliError>;

#[derive(thiserror::Error, Debug)]
pub enum CliError {
    #[error(transparent)]
    IoError(#[from] IoError),
    #[error("Fluvio client error")]
    ClientError(#[from] FluvioError),
    #[error("Fluvio cluster error")]
    ClusterError(#[from] ClusterError),
    #[error("Fluvio cluster pre install check error")]
    CheckError(#[from] CheckError),
    #[error("Kubernetes config error")]
    K8ConfigError(#[from] k8_config::ConfigError),
    #[error("Kubernetes client error")]
    K8ClientError(#[from] k8_client::ClientError),
    #[error("Package index error")]
    IndexError(#[from] fluvio_index::Error),
    #[error("Error finding executable")]
    WhichError(#[from] which::Error),
    #[error(transparent)]
    HttpError(#[from] HttpError),
    #[error("Invalid argument: {0}")]
    InvalidArg(String),
    #[error("Unknown error: {0}")]
    Other(String),
}

#[derive(thiserror::Error, Debug)]
#[error("Http Error: {}", inner)]
pub struct HttpError {
    inner: http_types::Error,
}

impl From<http_types::Error> for CliError {
    fn from(inner: http_types::Error) -> Self {
        Self::HttpError(HttpError { inner })
    }
}

impl CliError {
    pub fn invalid_arg<M: Into<String>>(reason: M) -> Self {
        Self::InvalidArg(reason.into())
    }

    pub fn into_report(self) -> color_eyre::Report {
        use color_eyre::{Report, Section};

        match self {
            e @ CliError::ClusterError(ClusterError::InstallK8(K8InstallError::PreCheck(CheckError::HelmError(HelmError::FailedToConnect)))) => {
                let report = Report::from(e);

                #[cfg(target_os = "macos")]
                let report = report.suggestion("Make sure you have run 'minikube start --driver=hyperkit'");
                #[cfg(not(target_os = "macos"))]
                let report = report.suggestion("Make sure you have run 'minikube start --driver=docker'");
                let report = report.suggestion("Make sure you have run 'minikube tunnel'");
                report
            }
            e => Report::from(e),
        }
    }
}

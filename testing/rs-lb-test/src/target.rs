use crate::config::TargetGroupConfiguration;

#[derive(Debug, thiserror::Error)]
pub enum TargetGroupCreationError {
    #[error("Failed to parse target: {0}")]
    ParsingTargetsFailed(String),
}

pub struct TargetGroup {
    pub targets: Vec<Target>,
}

impl TryFrom<&TargetGroupConfiguration> for TargetGroup {
    type Error = TargetGroupCreationError;

    fn try_from(value: &TargetGroupConfiguration) -> Result<Self, Self::Error> {
        let targets = value
            .targets
            .split(",")
            .map(Target::try_from)
            .collect::<Result<Vec<Target>, String>>()
            .map_err(TargetGroupCreationError::ParsingTargetsFailed)?;

        Ok(Self { targets })
    }
}

pub struct Target {
    pub hostname: String,
    pub port: u16,
    pub uri: String,
}

impl TryFrom<&str> for Target {
    type Error = String;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let (hostname, suffix) = value.split_once(":").ok_or(value.to_owned())?;
        let (port, uri) = if suffix.contains("/") {
            suffix.split_once("/").ok_or(value.to_owned())?
        } else {
            (suffix, "")
        };

        Ok(Self {
            hostname: hostname.to_owned(),
            port: port
                .parse()
                .map_err(|_| value.trim_matches('/').to_owned())?,
            uri: uri.to_owned(),
        })
    }
}

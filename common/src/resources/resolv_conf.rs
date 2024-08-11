use anyhow::bail;
use serde::{de::Error as SerdeError, Deserialize, Deserializer, Serialize};
use std::{net::IpAddr, str::FromStr};

#[derive(Clone, Debug, Eq, Hash, PartialEq, Serialize)]
pub struct SortlistPair(String);

impl<'de> Deserialize<'de> for SortlistPair {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let v = String::deserialize(deserializer)?;

        Self::from_str(&v).map_err(SerdeError::custom)
    }
}

impl FromStr for SortlistPair {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.split_once('/') {
            Some((prefix, suffix)) => {
                if let Err(error) = prefix.parse::<IpAddr>() {
                    bail!("invalid IP address: {}", error)
                }

                if let Err(error) = suffix.parse::<IpAddr>() {
                    bail!("invalid IP netmask after delimiter `/`: {}", error)
                }

                Ok(Self(s.to_string()))
            }
            None => match s.parse::<IpAddr>() {
                Ok(ip_address) => Ok(Self(ip_address.to_string())),
                Err(error) => {
                    bail!("invalid IP address: {}", error)
                }
            },
        }
    }
}

impl SortlistPair {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, Eq, Hash, PartialEq, Serialize)]
pub struct ResolverOption(String);

impl<'de> Deserialize<'de> for ResolverOption {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let v = String::deserialize(deserializer)?;

        Self::from_str(&v).map_err(SerdeError::custom)
    }
}

impl FromStr for ResolverOption {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let options: [&str; 64] = [
            "debug",
            "ndots:0",
            "ndots:1",
            "ndots:2",
            "ndots:3",
            "ndots:4",
            "ndots:5",
            "ndots:6",
            "ndots:7",
            "ndots:8",
            "ndots:9",
            "ndots:10",
            "ndots:11",
            "ndots:12",
            "ndots:13",
            "ndots:14",
            "ndots:15",
            "timeout:0",
            "timeout:1",
            "timeout:2",
            "timeout:3",
            "timeout:4",
            "timeout:5",
            "timeout:6",
            "timeout:7",
            "timeout:8",
            "timeout:9",
            "timeout:10",
            "timeout:11",
            "timeout:12",
            "timeout:13",
            "timeout:14",
            "timeout:15",
            "timeout:16",
            "timeout:17",
            "timeout:18",
            "timeout:19",
            "timeout:20",
            "timeout:21",
            "timeout:22",
            "timeout:23",
            "timeout:24",
            "timeout:25",
            "timeout:26",
            "timeout:27",
            "timeout:28",
            "timeout:29",
            "timeout:30",
            "attempts:0",
            "attempts:1",
            "attempts:2",
            "attempts:3",
            "attempts:4",
            "attempts:5",
            "rotate",
            "no-check-names",
            "inet6",
            "edns0",
            "single-request",
            "single-request-reopen",
            "no-tld-query",
            "use-vc",
            "no-reload",
            "trust-ad",
        ];

        if options.contains(&s) {
            Ok(Self(s.to_string()))
        } else {
            bail!("invalid resolver option `{}`", s)
        }
    }
}

impl ResolverOption {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

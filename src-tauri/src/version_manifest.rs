use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct VersionManifest {
    pub marketing_version: String,
    pub build_number: u64,
}

pub fn parse_version_manifest_str(raw: &str) -> Result<VersionManifest, String> {
    let parsed: VersionManifest =
        serde_json::from_str(raw).map_err(|err| format!("invalid version.json: {err}"))?;

    if !is_valid_semver(&parsed.marketing_version) {
        return Err(format!(
            "invalid marketingVersion {:?}; expected x.y.z",
            parsed.marketing_version
        ));
    }

    if parsed.build_number == 0 {
        return Err("buildNumber must be greater than zero".to_string());
    }

    Ok(parsed)
}

fn is_valid_semver(value: &str) -> bool {
    let mut parts = value.split('.');
    let (Some(major), Some(minor), Some(patch), None) =
        (parts.next(), parts.next(), parts.next(), parts.next())
    else {
        return false;
    };

    [major, minor, patch]
        .into_iter()
        .all(|part| !part.is_empty() && part.chars().all(|ch| ch.is_ascii_digit()))
}

#[cfg(test)]
mod tests {
    use super::{parse_version_manifest_str, VersionManifest};

    #[test]
    fn parses_valid_manifest() {
        let parsed =
            parse_version_manifest_str(r#"{"marketingVersion":"2.0.0","buildNumber":4}"#).unwrap();

        assert_eq!(
            parsed,
            VersionManifest {
                marketing_version: "2.0.0".to_string(),
                build_number: 4,
            }
        );
    }

    #[test]
    fn rejects_invalid_values() {
        assert!(parse_version_manifest_str(r#"{"marketingVersion":"2.0","buildNumber":4}"#).is_err());
        assert!(parse_version_manifest_str(r#"{"marketingVersion":"2.0.0","buildNumber":0}"#).is_err());
    }
}

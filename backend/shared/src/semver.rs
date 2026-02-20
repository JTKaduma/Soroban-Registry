use serde::{Deserialize, Serialize};

/// Semantic Versioning (SemVer) implementation
/// Supports parsing MAJOR.MINOR.PATCH and constraints like ^1.0.0, ~2.3.0

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SemVer {
    pub major: u64,
    pub minor: u64,
    pub patch: u64,
}

impl SemVer {
    pub fn parse(s: &str) -> Option<Self> {
        let parts: Vec<&str> = s.split('.').collect();
        if parts.len() != 3 {
            return None;
        }

        Some(SemVer {
            major: parts[0].parse().ok()?,
            minor: parts[1].parse().ok()?,
            patch: parts[2].parse().ok()?,
        })
    }
}

impl std::fmt::Display for SemVer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

impl PartialOrd for SemVer {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for SemVer {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.major
            .cmp(&other.major)
            .then(self.minor.cmp(&other.minor))
            .then(self.patch.cmp(&other.patch))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum VersionConstraint {
    Exact(SemVer),
    Caret(SemVer), // ^1.2.3 := >=1.2.3 <2.0.0
    Tilde(SemVer), // ~1.2.3 := >=1.2.3 <1.3.0
}

impl VersionConstraint {
    pub fn parse(s: &str) -> Option<Self> {
        let s = s.trim();
        if let Some(rest) = s.strip_prefix('^') {
            SemVer::parse(rest).map(VersionConstraint::Caret)
        } else if let Some(rest) = s.strip_prefix('~') {
            SemVer::parse(rest).map(VersionConstraint::Tilde)
        } else {
            SemVer::parse(s).map(VersionConstraint::Exact)
        }
    }

    pub fn matches(&self, version: &SemVer) -> bool {
        match self {
            VersionConstraint::Exact(req) => version == req,
            VersionConstraint::Caret(req) => {
                if version < req {
                    return false;
                }
                if req.major == 0 {
                    if req.minor == 0 {
                        // ^0.0.x is exact match
                        return version.patch == req.patch;
                    }
                    // ^0.x.y := >=0.x.y <0.(x+1).0
                    return version.major == 0 && version.minor == req.minor;
                }
                // ^1.x.y := >=1.x.y <2.0.0
                version.major == req.major
            }
            VersionConstraint::Tilde(req) => {
                if version < req {
                    return false;
                }
                // ~1.2.3 := >=1.2.3 <1.3.0
                version.major == req.major && version.minor == req.minor
            }
        }
    }
}

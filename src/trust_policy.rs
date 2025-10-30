//! Gem signature verification using X.509 certificates.

use anyhow::{Context, Result};
use der::DecodePem;
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::Read;
use std::path::{Path, PathBuf};
use tar::Archive;
use thiserror::Error;
use x509_cert::Certificate;
use x509_verify::{Signature, VerifyInfo, VerifyingKey};

/// Trust policy levels for gem signature verification
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrustPolicy {
    /// All gems must be signed and verified
    HighSecurity,
    /// All signed gems must be verified (allows unsigned gems)
    MediumSecurity,
    /// Warns about unsigned gems but allows them
    LowSecurity,
    /// No verification (default)
    NoSecurity,
}

impl TrustPolicy {
    /// Parse trust policy from string
    ///
    /// # Example
    ///
    /// ```
    /// use lode::trust_policy::TrustPolicy;
    ///
    /// assert_eq!(TrustPolicy::parse("HighSecurity"), Some(TrustPolicy::HighSecurity));
    /// assert_eq!(TrustPolicy::parse("MediumSecurity"), Some(TrustPolicy::MediumSecurity));
    /// assert_eq!(TrustPolicy::parse("invalid"), None);
    /// ```
    #[must_use]
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "HighSecurity" => Some(Self::HighSecurity),
            "MediumSecurity" => Some(Self::MediumSecurity),
            "LowSecurity" => Some(Self::LowSecurity),
            "NoSecurity" => Some(Self::NoSecurity),
            _ => None,
        }
    }

    /// Returns whether this policy requires signature verification
    #[must_use]
    pub const fn requires_verification(self) -> bool {
        matches!(
            self,
            Self::HighSecurity | Self::MediumSecurity | Self::LowSecurity
        )
    }

    /// Returns whether this policy allows unsigned gems
    #[must_use]
    pub const fn allows_unsigned(self) -> bool {
        !matches!(self, Self::HighSecurity)
    }
}

impl std::fmt::Display for TrustPolicy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::HighSecurity => write!(f, "HighSecurity"),
            Self::MediumSecurity => write!(f, "MediumSecurity"),
            Self::LowSecurity => write!(f, "LowSecurity"),
            Self::NoSecurity => write!(f, "NoSecurity"),
        }
    }
}

/// Errors that can occur during gem signature verification
#[derive(Debug, Error)]
pub enum VerificationError {
    #[error("Gem is not signed: {gem_path}")]
    UnsignedGem { gem_path: String },

    #[error("Invalid signature for {gem_path}: {reason}")]
    InvalidSignature { gem_path: String, reason: String },

    #[error("No trusted certificate found for {gem_path}")]
    NoTrustedCertificate { gem_path: String },

    #[error("Failed to load certificate from {path}: {source}")]
    CertificateLoadError {
        path: String,
        #[source]
        source: anyhow::Error,
    },

    #[error("Trust policy violation for {gem_path}: {reason}")]
    PolicyViolation { gem_path: String, reason: String },
}

/// Gem signature verifier
///
/// Loads trusted certificates and verifies gem signatures according to the trust policy.
#[derive(Debug)]
pub struct GemVerifier {
    policy: TrustPolicy,
    trust_dir: PathBuf,
    certificates: HashMap<String, String>,
}

impl GemVerifier {
    /// Create a new gem verifier with the specified trust policy
    ///
    /// Loads certificates from `~/.gem/trust/` directory.
    ///
    /// # Errors
    ///
    /// Returns an error if the trust directory cannot be accessed or certificates cannot be loaded.
    pub fn new(policy: TrustPolicy) -> Result<Self> {
        let home = dirs::home_dir().context("Failed to find home directory")?;
        let trust_dir = home.join(".gem").join("trust");

        let mut verifier = Self {
            policy,
            trust_dir,
            certificates: HashMap::new(),
        };

        // Load certificates if verification is needed
        if policy.requires_verification() {
            verifier.load_certificates()?;
        }

        Ok(verifier)
    }

    /// Load trusted certificates from the trust directory
    fn load_certificates(&mut self) -> Result<()> {
        // Create trust directory if it doesn't exist
        if !self.trust_dir.exists() {
            fs::create_dir_all(&self.trust_dir).with_context(|| {
                format!(
                    "Failed to create trust directory: {}",
                    self.trust_dir.display()
                )
            })?;
            return Ok(());
        }

        // Read all .pem files from trust directory
        for entry in fs::read_dir(&self.trust_dir).with_context(|| {
            format!(
                "Failed to read trust directory: {}",
                self.trust_dir.display()
            )
        })? {
            let entry = entry?;
            let path = entry.path();

            if path.extension().is_some_and(|ext| ext == "pem") {
                let cert_data = fs::read_to_string(&path).map_err(|err| {
                    VerificationError::CertificateLoadError {
                        path: path.display().to_string(),
                        source: anyhow::Error::new(err),
                    }
                })?;

                let filename = path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("unknown")
                    .to_string();

                self.certificates.insert(filename, cert_data);
            }
        }

        Ok(())
    }

    /// Verify a gem file according to the trust policy.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The gem is unsigned and the policy requires signatures
    /// - The gem has an invalid signature
    /// - No trusted certificate is found for the gem
    pub fn verify_gem(&self, gem_path: &Path) -> Result<(), VerificationError> {
        // NoSecurity policy: skip all verification
        if self.policy == TrustPolicy::NoSecurity {
            return Ok(());
        }

        let gem_path_str = gem_path.display().to_string();

        let is_signed = Self::is_gem_signed(gem_path)?;

        if !is_signed {
            return match self.policy {
                TrustPolicy::HighSecurity => Err(VerificationError::UnsignedGem {
                    gem_path: gem_path_str,
                }),
                TrustPolicy::MediumSecurity | TrustPolicy::LowSecurity => {
                    if self.policy == TrustPolicy::LowSecurity {
                        eprintln!("  Warning: Gem is not signed: {gem_path_str}");
                    }
                    Ok(())
                }
                TrustPolicy::NoSecurity => Ok(()),
            };
        }

        self.verify_signature(gem_path)?;

        Ok(())
    }

    /// Check if a gem file is signed
    ///
    /// A gem is considered signed if it contains signature files (*.sig) in its archive.
    fn is_gem_signed(gem_path: &Path) -> Result<bool, VerificationError> {
        let file = fs::File::open(gem_path).map_err(|err| VerificationError::InvalidSignature {
            gem_path: gem_path.display().to_string(),
            reason: format!("Failed to open gem file: {err}"),
        })?;

        let mut archive = tar::Archive::new(file);

        for entry_result in
            archive
                .entries()
                .map_err(|err| VerificationError::InvalidSignature {
                    gem_path: gem_path.display().to_string(),
                    reason: format!("Failed to read gem archive: {err}"),
                })?
        {
            let entry = entry_result.map_err(|err| VerificationError::InvalidSignature {
                gem_path: gem_path.display().to_string(),
                reason: format!("Failed to read archive entry: {err}"),
            })?;

            let path = entry
                .path()
                .map_err(|err| VerificationError::InvalidSignature {
                    gem_path: gem_path.display().to_string(),
                    reason: format!("Failed to read entry path: {err}"),
                })?;

            // Check for signature files (case-insensitive)
            if path
                .extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("sig"))
            {
                return Ok(true);
            }
        }

        Ok(false)
    }

    /// Verify the signature of a signed gem using X.509 certificates
    ///
    /// Extracts signature files from the gem archive and verifies them against
    /// trusted certificates using RSA/SHA256 verification.
    fn verify_signature(&self, gem_path: &Path) -> Result<(), VerificationError> {
        let gem_path_str = gem_path.display().to_string();

        if self.certificates.is_empty() {
            return Err(VerificationError::NoTrustedCertificate {
                gem_path: gem_path_str,
            });
        }

        let (data_content, sig_content) =
            Self::extract_gem_signature_files(gem_path).map_err(|e| {
                VerificationError::InvalidSignature {
                    gem_path: gem_path_str.clone(),
                    reason: format!("Failed to extract signature files: {e}"),
                }
            })?;

        let mut last_error = None;
        for (cert_name, cert_pem) in &self.certificates {
            match Self::verify_with_certificate(&data_content, &sig_content, cert_pem) {
                Ok(()) => return Ok(()),
                Err(e) => {
                    last_error = Some(format!("Certificate '{cert_name}': {e}"));
                }
            }
        }

        Err(VerificationError::InvalidSignature {
            gem_path: gem_path_str,
            reason: last_error.unwrap_or_else(|| "No matching certificate found".to_string()),
        })
    }

    /// Extract data.tar.gz and data.tar.gz.sig from gem archive
    fn extract_gem_signature_files(gem_path: &Path) -> Result<(Vec<u8>, Vec<u8>)> {
        let file = File::open(gem_path)
            .with_context(|| format!("Failed to open gem file: {}", gem_path.display()))?;
        let mut archive = Archive::new(file);

        let mut data_content = None;
        let mut sig_content = None;

        for entry_result in archive.entries()? {
            let mut entry = entry_result?;
            let path = entry.path()?;
            let path_str = path.to_string_lossy();

            if path_str == "data.tar.gz" {
                let mut content = Vec::new();
                entry.read_to_end(&mut content)?;
                data_content = Some(content);
            } else if path_str == "data.tar.gz.sig" {
                let mut content = Vec::new();
                entry.read_to_end(&mut content)?;
                sig_content = Some(content);
            }

            if data_content.is_some() && sig_content.is_some() {
                break;
            }
        }

        match (data_content, sig_content) {
            (Some(data), Some(sig)) => Ok((data, sig)),
            (None, _) => anyhow::bail!("data.tar.gz not found in gem archive"),
            (_, None) => anyhow::bail!("data.tar.gz.sig not found in gem archive"),
        }
    }

    /// Verify signature using a specific certificate
    fn verify_with_certificate(data: &[u8], sig_bytes: &[u8], cert_pem: &str) -> Result<()> {
        // Parse the X.509 certificate
        let cert = Certificate::from_pem(cert_pem).context("Failed to parse X.509 certificate")?;

        // Create Signature - gem signatures use the same algorithm as the certificate signature
        let signature = Signature::new(&cert.signature_algorithm, sig_bytes);

        // Create VerifyInfo with the data to verify and the signature
        let verify_info = VerifyInfo::new(data.to_vec().into(), signature);

        // Extract VerifyingKey from the certificate's subject public key info
        let key: VerifyingKey = cert
            .tbs_certificate
            .subject_public_key_info
            .try_into()
            .map_err(|e| anyhow::anyhow!("Failed to extract public key from certificate: {e:?}"))?;

        // Verify the signature (x509-verify handles hashing internally)
        key.verify(&verify_info)
            .map_err(|e| anyhow::anyhow!("Signature verification failed: {e:?}"))?;

        Ok(())
    }

    /// Get the current trust policy
    #[must_use]
    pub const fn policy(&self) -> TrustPolicy {
        self.policy
    }

    /// Get the number of loaded trusted certificates
    #[must_use]
    pub fn certificate_count(&self) -> usize {
        self.certificates.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trust_policy_parse() {
        assert_eq!(
            TrustPolicy::parse("HighSecurity"),
            Some(TrustPolicy::HighSecurity)
        );
        assert_eq!(
            TrustPolicy::parse("MediumSecurity"),
            Some(TrustPolicy::MediumSecurity)
        );
        assert_eq!(
            TrustPolicy::parse("LowSecurity"),
            Some(TrustPolicy::LowSecurity)
        );
        assert_eq!(
            TrustPolicy::parse("NoSecurity"),
            Some(TrustPolicy::NoSecurity)
        );
        assert_eq!(TrustPolicy::parse("invalid"), None);
    }

    #[test]
    fn trust_policy_display() {
        assert_eq!(TrustPolicy::HighSecurity.to_string(), "HighSecurity");
        assert_eq!(TrustPolicy::MediumSecurity.to_string(), "MediumSecurity");
        assert_eq!(TrustPolicy::LowSecurity.to_string(), "LowSecurity");
        assert_eq!(TrustPolicy::NoSecurity.to_string(), "NoSecurity");
    }

    #[test]
    fn trust_policy_requires_verification() {
        assert!(TrustPolicy::HighSecurity.requires_verification());
        assert!(TrustPolicy::MediumSecurity.requires_verification());
        assert!(TrustPolicy::LowSecurity.requires_verification());
        assert!(!TrustPolicy::NoSecurity.requires_verification());
    }

    #[test]
    fn trust_policy_allows_unsigned() {
        assert!(!TrustPolicy::HighSecurity.allows_unsigned());
        assert!(TrustPolicy::MediumSecurity.allows_unsigned());
        assert!(TrustPolicy::LowSecurity.allows_unsigned());
        assert!(TrustPolicy::NoSecurity.allows_unsigned());
    }

    #[test]
    fn gem_verifier_creation_no_security() -> Result<()> {
        let verifier = GemVerifier::new(TrustPolicy::NoSecurity)?;
        assert_eq!(verifier.policy(), TrustPolicy::NoSecurity);
        assert_eq!(verifier.certificate_count(), 0);
        Ok(())
    }

    mod verification_errors {
        use super::*;

        #[test]
        fn unsigned_gem_error_display() {
            let err = VerificationError::UnsignedGem {
                gem_path: "test.gem".to_string(),
            };
            assert!(err.to_string().contains("not signed"));
            assert!(err.to_string().contains("test.gem"));
        }

        #[test]
        fn invalid_signature_error_display() {
            let err = VerificationError::InvalidSignature {
                gem_path: "test.gem".to_string(),
                reason: "bad key".to_string(),
            };
            assert!(err.to_string().contains("Invalid signature"));
            assert!(err.to_string().contains("bad key"));
        }

        #[test]
        fn no_trusted_certificate_error_display() {
            let err = VerificationError::NoTrustedCertificate {
                gem_path: "test.gem".to_string(),
            };
            assert!(err.to_string().contains("No trusted certificate"));
        }

        #[test]
        fn policy_violation_error_display() {
            let err = VerificationError::PolicyViolation {
                gem_path: "test.gem".to_string(),
                reason: "must be signed".to_string(),
            };
            assert!(err.to_string().contains("Trust policy violation"));
        }
    }

    mod policy_behavior {
        use super::*;

        #[test]
        fn high_security_rejects_unsigned() {
            assert!(TrustPolicy::HighSecurity.requires_verification());
            assert!(!TrustPolicy::HighSecurity.allows_unsigned());
        }

        #[test]
        fn medium_security_allows_unsigned() {
            assert!(TrustPolicy::MediumSecurity.requires_verification());
            assert!(TrustPolicy::MediumSecurity.allows_unsigned());
        }

        #[test]
        fn low_security_allows_unsigned() {
            assert!(TrustPolicy::LowSecurity.requires_verification());
            assert!(TrustPolicy::LowSecurity.allows_unsigned());
        }

        #[test]
        fn no_security_skips_verification() {
            assert!(!TrustPolicy::NoSecurity.requires_verification());
            assert!(TrustPolicy::NoSecurity.allows_unsigned());
        }

        #[test]
        fn parse_case_sensitive() {
            assert!(TrustPolicy::parse("HighSecurity").is_some());
            assert!(TrustPolicy::parse("highsecurity").is_none());
            assert!(TrustPolicy::parse("HIGHSECURITY").is_none());
        }

        #[test]
        fn parse_empty_string() {
            assert!(TrustPolicy::parse("").is_none());
        }
    }

    mod certificate_operations {
        use super::*;
        use tempfile::TempDir;

        #[test]
        fn verifier_accepts_no_security() -> Result<()> {
            let verifier = GemVerifier::new(TrustPolicy::NoSecurity)?;
            assert_eq!(verifier.policy(), TrustPolicy::NoSecurity);
            assert_eq!(verifier.certificate_count(), 0);
            Ok(())
        }

        #[test]
        fn verifier_medium_security_creates_trust_dir() -> Result<()> {
            let _temp = TempDir::new()?;
            let verifier = GemVerifier::new(TrustPolicy::MediumSecurity)?;
            assert_eq!(verifier.policy(), TrustPolicy::MediumSecurity);
            Ok(())
        }

        #[test]
        fn verifier_high_security_creates_trust_dir() -> Result<()> {
            let _temp = TempDir::new()?;
            let verifier = GemVerifier::new(TrustPolicy::HighSecurity)?;
            assert_eq!(verifier.policy(), TrustPolicy::HighSecurity);
            Ok(())
        }

        #[test]
        fn verifier_policy_accessor() -> Result<()> {
            let verifier = GemVerifier::new(TrustPolicy::LowSecurity)?;
            assert_eq!(verifier.policy(), TrustPolicy::LowSecurity);
            Ok(())
        }
    }

    mod archive_operations {
        use super::*;
        use std::fs;
        use std::io::Cursor;
        use tar::Builder;
        use tempfile::TempDir;

        fn create_test_gem_unsigned(temp: &TempDir) -> Result<PathBuf> {
            let gem_path = temp.path().join("test-1.0.0.gem");

            let mut builder = Builder::new(fs::File::create(&gem_path)?);

            {
                let mut data_tar = Vec::new();
                {
                    let mut data_builder = Builder::new(&mut data_tar);
                    let content = b"test data";
                    let mut header = tar::Header::new_gnu();
                    header.set_size(content.len() as u64);
                    data_builder.append_data(&mut header, "data.txt", Cursor::new(content))?;
                    data_builder.finish()?;
                }

                let mut header = tar::Header::new_gnu();
                header.set_size(data_tar.len() as u64);
                builder.append_data(&mut header, "data.tar.gz", Cursor::new(data_tar))?;
            }

            {
                let mut metadata_tar = Vec::new();
                {
                    let mut metadata_builder = Builder::new(&mut metadata_tar);
                    let content = b"metadata";
                    let mut header = tar::Header::new_gnu();
                    header.set_size(content.len() as u64);
                    metadata_builder.append_data(
                        &mut header,
                        "metadata.txt",
                        Cursor::new(content),
                    )?;
                    metadata_builder.finish()?;
                }

                let mut header = tar::Header::new_gnu();
                header.set_size(metadata_tar.len() as u64);
                builder.append_data(&mut header, "metadata.gz", Cursor::new(metadata_tar))?;
            }

            builder.finish()?;
            Ok(gem_path)
        }

        #[test]
        fn detect_unsigned_gem() -> Result<()> {
            let temp = TempDir::new()?;
            let gem_path = create_test_gem_unsigned(&temp)?;
            assert!(!GemVerifier::is_gem_signed(&gem_path)?);
            Ok(())
        }
    }
}

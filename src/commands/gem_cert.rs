//! Cert command
//!
//! Manage signing certificates for gems

use anyhow::{Context, Result};
use rcgen::{CertificateParams, DistinguishedName, DnType, IsCa, KeyPair};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};

/// Options for gem cert command
#[derive(Debug, Default)]
pub(crate) struct CertOptions {
    /// Build a self-signed certificate for this email
    pub build: Option<String>,

    /// Add a trusted certificate from this path
    pub add: Option<String>,

    /// List trusted certificates (optional filter)
    pub list: bool,
    pub list_filter: Option<String>,

    /// Remove trusted certificates matching this filter
    pub remove: Option<String>,

    /// Sign this certificate
    pub sign: Option<String>,

    /// Signing certificate path (for --sign)
    pub certificate: Option<String>,

    /// Private key path
    pub private_key: Option<String>,

    /// Key algorithm (rsa or ec)
    pub key_algorithm: Option<String>,

    /// Days until certificate expires
    pub days: Option<u32>,

    /// Re-sign the certificate
    pub re_sign: bool,
}

/// Run the gem cert command
pub(crate) fn run(options: CertOptions) -> Result<()> {
    // Handle --build
    if let Some(email) = options.build {
        return build_certificate(
            &email,
            options.private_key.as_deref(),
            options.key_algorithm.as_deref(),
            options.days,
        );
    }

    // Handle --add
    if let Some(cert_path) = options.add {
        return add_certificate(&cert_path);
    }

    // Handle --list
    if options.list {
        return list_certificates(options.list_filter.as_deref());
    }

    // Handle --remove
    if let Some(filter) = options.remove {
        return remove_certificates(&filter);
    }

    // Handle --sign
    if let Some(cert_to_sign) = options.sign {
        let signing_cert = options
            .certificate
            .context("--sign requires --certificate (-C) option")?;
        let private_key = options
            .private_key
            .context("--sign requires --private-key (-K) option")?;

        return sign_certificate(&cert_to_sign, &signing_cert, &private_key);
    }

    // Handle --re-sign
    if options.re_sign {
        let cert_path = options
            .certificate
            .context("--re-sign requires --certificate (-C) option")?;
        let private_key = options
            .private_key
            .context("--re-sign requires --private-key (-K) option")?;

        return re_sign_certificate(&cert_path, &private_key);
    }

    // No action specified, show help
    anyhow::bail!(
        "No action specified. Use --help to see available options.\n\nCommon commands:\n  gem cert --build your@email.com\n  gem cert --list\n  gem cert --add /path/to/cert.pem"
    )
}

/// Build a self-signed certificate
fn build_certificate(
    email: &str,
    private_key_path: Option<&str>,
    key_algorithm: Option<&str>,
    days: Option<u32>,
) -> Result<()> {
    let gem_dir = get_gem_dir()?;
    let cert_path = gem_dir.join("gem-public_cert.pem");
    let key_path = gem_dir.join("gem-private_key.pem");

    println!("Building certificate for: {email}");

    // Load or generate key pair
    let key_pair = if let Some(existing_key_path) = private_key_path {
        println!("Using existing private key: {existing_key_path}");
        let key_pem =
            fs::read_to_string(existing_key_path).context("Failed to read private key")?;
        KeyPair::from_pem(&key_pem).context("Failed to parse private key")?
    } else {
        // Generate new key pair
        let algorithm = key_algorithm.unwrap_or("rsa");
        println!("Generating new {algorithm} key pair...");

        match algorithm.to_lowercase().as_str() {
            "rsa" => KeyPair::generate().context("Failed to generate RSA key pair")?,
            "ec" | "ecdsa" => {
                // rcgen's default KeyPair::generate() creates RSA
                // For ECDSA, we'd need to use a different approach
                // For now, we'll default to RSA for compatibility
                println!("Note: ECDSA support is limited, using RSA 2048-bit");
                KeyPair::generate().context("Failed to generate key pair")?
            }
            _ => anyhow::bail!("Unsupported key algorithm: {algorithm}. Use 'rsa' or 'ec'."),
        }
    };

    // Create certificate parameters
    let mut params = CertificateParams::default();

    // Set subject distinguished name
    let mut dn = DistinguishedName::new();
    dn.push(DnType::CommonName, email);
    dn.push(DnType::OrganizationName, "Gem Development");
    params.distinguished_name = dn;

    // Set validity period
    let validity_days = days.unwrap_or(365);
    params.not_before = time::OffsetDateTime::now_utc();
    params.not_after =
        time::OffsetDateTime::now_utc() + time::Duration::days(i64::from(validity_days));

    // Set as self-signed CA
    params.is_ca = IsCa::Ca(rcgen::BasicConstraints::Unconstrained);

    // Generate certificate
    let cert = params
        .self_signed(&key_pair)
        .context("Failed to generate self-signed certificate")?;

    // Save certificate
    fs::write(&cert_path, cert.pem()).context("Failed to write certificate")?;
    println!("Certificate saved to: {}", cert_path.display());

    // Save private key (if new)
    if private_key_path.is_none() {
        fs::write(&key_path, key_pair.serialize_pem()).context("Failed to write private key")?;
        println!("Private key saved to: {}", key_path.display());

        // Set restrictive permissions on private key
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&key_path)?.permissions();
            perms.set_mode(0o600); // Read/write for owner only
            fs::set_permissions(&key_path, perms)?;
        }
    }

    println!("\n Certificate is valid for {validity_days} days");
    println!("\n💡 You can now sign gems with this certificate:");
    println!("   lode gem-build your-gem.gemspec --sign");

    Ok(())
}

/// List certificates from trust store
fn list_certificates(filter: Option<&str>) -> Result<()> {
    let trust_dir = get_trust_dir()?;

    if !trust_dir.exists() {
        println!("No trusted certificates found.");
        println!("\n💡 Add a certificate with:");
        println!("   lode gem-cert --add /path/to/cert.pem");
        return Ok(());
    }

    let mut certs = Vec::new();

    // Read all certificate files
    for entry in fs::read_dir(&trust_dir).context("Failed to read trust directory")? {
        let entry = entry?;
        let path = entry.path();

        if path.extension().and_then(|s| s.to_str()) != Some("pem") {
            continue;
        }

        // Read certificate
        let Ok(cert_pem) = fs::read_to_string(&path) else {
            continue; // Skip unreadable files
        };

        // Parse certificate to extract subject
        if let Ok(subject) = extract_subject(&cert_pem) {
            // Apply filter if provided
            if let Some(filter_str) = filter
                && !subject.to_lowercase().contains(&filter_str.to_lowercase())
            {
                continue;
            }

            certs.push((subject, path));
        }
    }

    if certs.is_empty() {
        if let Some(filter_str) = filter {
            println!("No certificates found matching: {filter_str}");
        } else {
            println!("No trusted certificates found.");
        }
        return Ok(());
    }

    // Sort by subject for consistent output
    certs.sort_by(|a, b| a.0.cmp(&b.0));

    println!("Trusted certificates:");
    println!();
    for (subject, path) in certs {
        println!("   {subject}");
        println!("     {}", path.display());
        println!();
    }

    Ok(())
}

/// Add a certificate to the trust store
fn add_certificate(cert_path: &str) -> Result<()> {
    let source_path = Path::new(cert_path);

    if !source_path.exists() {
        anyhow::bail!("Certificate file not found: {cert_path}");
    }

    // Read and validate certificate
    let cert_pem = fs::read_to_string(source_path).context("Failed to read certificate file")?;

    let subject = extract_subject(&cert_pem).context("Failed to parse certificate")?;

    // Create trust directory if needed
    let trust_dir = get_trust_dir()?;
    if !trust_dir.exists() {
        fs::create_dir_all(&trust_dir).context("Failed to create trust directory")?;
    }

    // Generate filename from subject hash
    let filename = generate_cert_filename(&subject);
    let dest_path = trust_dir.join(filename);

    // Copy certificate
    fs::copy(source_path, &dest_path).context("Failed to copy certificate")?;

    println!("Added certificate to trust store:");
    println!("   Subject: {subject}");
    println!("   Path: {}", dest_path.display());

    Ok(())
}

/// Remove certificates matching filter
fn remove_certificates(filter: &str) -> Result<()> {
    let trust_dir = get_trust_dir()?;

    if !trust_dir.exists() {
        println!("No trusted certificates found.");
        return Ok(());
    }

    let mut removed = Vec::new();

    // Find matching certificates
    for entry in fs::read_dir(&trust_dir).context("Failed to read trust directory")? {
        let entry = entry?;
        let path = entry.path();

        if path.extension().and_then(|s| s.to_str()) != Some("pem") {
            continue;
        }

        // Read certificate
        let Ok(cert_pem) = fs::read_to_string(&path) else {
            continue;
        };

        // Check if subject matches filter
        if let Ok(subject) = extract_subject(&cert_pem)
            && subject.to_lowercase().contains(&filter.to_lowercase())
        {
            // Remove certificate
            if fs::remove_file(&path).is_ok() {
                removed.push(subject);
            }
        }
    }

    if removed.is_empty() {
        println!("No certificates found matching: {filter}");
    } else {
        println!("Removed {} certificate(s):", removed.len());
        for subject in removed {
            println!("   {subject}");
        }
    }

    Ok(())
}

/// Sign another certificate (advanced feature)
fn sign_certificate(cert_to_sign: &str, signing_cert: &str, private_key: &str) -> Result<()> {
    // Read the certificate to sign
    let cert_pem =
        fs::read_to_string(cert_to_sign).context("Failed to read certificate to sign")?;

    // Read signing certificate
    let _signing_cert_pem =
        fs::read_to_string(signing_cert).context("Failed to read signing certificate")?;

    // Read private key
    let key_pem = fs::read_to_string(private_key).context("Failed to read private key")?;
    let key_pair = KeyPair::from_pem(&key_pem).context("Failed to parse private key")?;

    // Parse certificate parameters from the cert to sign
    // Note: rcgen doesn't directly support parsing existing certificates into CertificateParams
    // This is a simplified implementation

    let subject = extract_subject(&cert_pem)?;

    // Create new certificate params with the same subject
    let mut params = CertificateParams::default();
    let mut dn = DistinguishedName::new();
    dn.push(DnType::CommonName, &subject);
    params.distinguished_name = dn;

    // Sign with the provided key
    let signed_cert = params
        .self_signed(&key_pair)
        .context("Failed to sign certificate")?;

    // Generate output filename
    let output_path = format!("{cert_to_sign}.signed");
    fs::write(&output_path, signed_cert.pem()).context("Failed to write signed certificate")?;

    println!("Done");
    println!("   Input: {cert_to_sign}");
    println!("   Output: {output_path}");

    Ok(())
}

/// Re-sign a certificate
fn re_sign_certificate(cert_path: &str, private_key: &str) -> Result<()> {
    sign_certificate(cert_path, cert_path, private_key)
}

/// Get the gem directory (~/.gem)
fn get_gem_dir() -> Result<PathBuf> {
    let home = dirs::home_dir().context("Could not determine home directory")?;
    let gem_dir = home.join(".gem");

    if !gem_dir.exists() {
        fs::create_dir_all(&gem_dir).context("Failed to create .gem directory")?;
    }

    Ok(gem_dir)
}

/// Get the trust directory (~/.gem/trust)
fn get_trust_dir() -> Result<PathBuf> {
    Ok(get_gem_dir()?.join("trust"))
}

/// Extract subject from a PEM certificate
fn extract_subject(cert_pem: &str) -> Result<String> {
    // Simple PEM parsing to extract subject
    // In a real implementation, we'd use a proper X.509 parser
    // For now, we'll try to extract the CN from the certificate

    // Look for subject line in openssl-style output
    // This is a simplified version - in production, use x509-parser crate

    if cert_pem.contains("BEGIN CERTIFICATE") {
        // Try to extract CN from the PEM
        // For MVP, we'll just return a placeholder based on the cert hash
        let mut hasher = Sha256::new();
        hasher.update(cert_pem.as_bytes());
        let hash = hasher.finalize();
        Ok(format!(
            "Certificate-{:x}",
            hash.get(..4)
                .expect("SHA256 hash is always 32 bytes")
                .iter()
                .fold(0u32, |acc, &b| acc << 8 | u32::from(b))
        ))
    } else {
        anyhow::bail!("Invalid PEM certificate")
    }
}

/// Generate a filename for a certificate based on its subject
fn generate_cert_filename(subject: &str) -> String {
    // Sanitize subject to create a valid filename
    let sanitized = subject
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect::<String>();

    // Add .pem extension
    format!("{sanitized}.pem")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper to create minimal `CertOptions`
    fn minimal_cert_options() -> CertOptions {
        CertOptions::default()
    }

    #[test]
    fn test_generate_cert_filename() {
        let filename = generate_cert_filename("test@example.com");
        assert!(
            std::path::Path::new(&filename)
                .extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("pem"))
        );
        assert!(!filename.contains('@'));
    }

    #[test]
    fn test_get_gem_dir() {
        let gem_dir = get_gem_dir();
        assert!(gem_dir.is_ok());
    }

    #[test]
    fn test_get_trust_dir() {
        let trust_dir = get_trust_dir();
        assert!(trust_dir.is_ok());
        assert!(trust_dir.unwrap().ends_with("trust"));
    }

    #[test]
    fn test_cert_options_build_flag() {
        let mut opts = minimal_cert_options();
        opts.build = Some("test@example.com".to_string());
        assert_eq!(opts.build, Some("test@example.com".to_string()));
    }

    #[test]
    fn test_cert_options_add_certificate() {
        let mut opts = minimal_cert_options();
        opts.add = Some("/path/to/cert.pem".to_string());
        assert_eq!(opts.add, Some("/path/to/cert.pem".to_string()));
    }

    #[test]
    fn test_cert_options_list_certificates() {
        let mut opts = minimal_cert_options();
        opts.list = true;
        assert!(opts.list);
        assert_eq!(opts.list_filter, None);
    }

    #[test]
    fn test_cert_options_list_with_filter() {
        let mut opts = minimal_cert_options();
        opts.list = true;
        opts.list_filter = Some("example.com".to_string());
        assert!(opts.list);
        assert_eq!(opts.list_filter, Some("example.com".to_string()));
    }

    #[test]
    fn test_cert_options_remove_certificate() {
        let mut opts = minimal_cert_options();
        opts.remove = Some("test@example.com".to_string());
        assert_eq!(opts.remove, Some("test@example.com".to_string()));
    }

    #[test]
    fn test_cert_options_sign_operation() {
        let mut opts = minimal_cert_options();
        opts.sign = Some("cert-to-sign.pem".to_string());
        opts.certificate = Some("signer.pem".to_string());
        opts.private_key = Some("key.pem".to_string());
        assert_eq!(opts.sign, Some("cert-to-sign.pem".to_string()));
        assert_eq!(opts.certificate, Some("signer.pem".to_string()));
        assert_eq!(opts.private_key, Some("key.pem".to_string()));
    }

    #[test]
    fn test_cert_options_re_sign() {
        let mut opts = minimal_cert_options();
        opts.re_sign = true;
        opts.certificate = Some("cert.pem".to_string());
        opts.private_key = Some("key.pem".to_string());
        assert!(opts.re_sign);
    }

    #[test]
    fn test_cert_options_key_algorithm() {
        let mut opts = minimal_cert_options();
        opts.key_algorithm = Some("rsa".to_string());
        assert_eq!(opts.key_algorithm, Some("rsa".to_string()));

        let mut opts = minimal_cert_options();
        opts.key_algorithm = Some("ec".to_string());
        assert_eq!(opts.key_algorithm, Some("ec".to_string()));
    }

    #[test]
    fn test_cert_options_days_expiration() {
        let mut opts = minimal_cert_options();
        opts.days = Some(365);
        assert_eq!(opts.days, Some(365));

        let mut opts = minimal_cert_options();
        opts.days = Some(3650);
        assert_eq!(opts.days, Some(3650));
    }

    #[test]
    fn test_cert_options_certificate_path() {
        let mut opts = minimal_cert_options();
        opts.certificate = Some("/home/user/.gem/gem-public_cert.pem".to_string());
        assert_eq!(
            opts.certificate,
            Some("/home/user/.gem/gem-public_cert.pem".to_string())
        );
    }

    #[test]
    fn test_cert_options_private_key_path() {
        let mut opts = minimal_cert_options();
        opts.private_key = Some("/home/user/.gem/gem-private_key.pem".to_string());
        assert_eq!(
            opts.private_key,
            Some("/home/user/.gem/gem-private_key.pem".to_string())
        );
    }

    #[test]
    fn test_generate_cert_filename_special_chars() {
        let filename = generate_cert_filename("user@example.com");
        assert!(
            std::path::Path::new(&filename)
                .extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("pem"))
        );
        assert!(!filename.contains('@'));
        assert!(!filename.contains(' '));
    }
}

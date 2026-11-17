//! Misc helpers: hex strings and OS detection.

use std::sync::OnceLock;

use crate::base::generated::steam_language::EOSType;
use crate::error::SteamError;

/// Misc helpers used throughout the library.
#[derive(Debug)]
pub(crate) struct Utils;

impl Utils {
    /// Encodes bytes as a lowercase hex string.
    pub(crate) fn encode_hex_string(input: &[u8]) -> String {
        hex::encode(input)
    }

    /// Decodes a hex string. `None` in, `None` out; invalid hex is an error.
    pub(crate) fn decode_hex_string(hex: Option<&str>) -> Result<Option<Vec<u8>>, SteamError> {
        let Some(hex) = hex else {
            return Ok(None);
        };

        hex::decode(hex)
            .map(Some)
            .map_err(|error| SteamError::InvalidArgument(format!("Invalid hex string: {error}")))
    }

    /// The operating system we're running on. Cached, since it can't change.
    pub(crate) fn get_os_type() -> EOSType {
        static OS_TYPE: OnceLock<EOSType> = OnceLock::new();

        *OS_TYPE.get_or_init(detect_os_type)
    }
}

/// Encodes bytes as a lowercase hex string.
pub(crate) fn encode_hex_string(input: &[u8]) -> String {
    Utils::encode_hex_string(input)
}

/// Decodes a hex string. Same as [`Utils::decode_hex_string`].
pub(crate) fn decode_hex_string(hex: Option<&str>) -> Result<Option<Vec<u8>>, SteamError> {
    Utils::decode_hex_string(hex)
}

/// The operating system we're running on. Same as [`Utils::get_os_type`].
pub(crate) fn get_os_type() -> EOSType {
    Utils::get_os_type()
}

/// Detects the operating system and maps it to Valve's `EOSType`. The standard
/// library doesn't expose OS or kernel versions, so each `detect_*` function
/// asks the platform's own tools.
fn detect_os_type() -> EOSType {
    #[cfg(target_os = "windows")]
    {
        detect_windows_os_type()
    }

    #[cfg(target_os = "macos")]
    {
        detect_macos_os_type()
    }

    #[cfg(target_os = "linux")]
    {
        detect_linux_os_type()
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    {
        EOSType::Unknown
    }
}

/// Maps a Windows version to Valve's `EOSType`: by major, then minor version,
/// and for Windows 10+ by build number (which tells Windows 11 apart).
#[cfg(target_os = "windows")]
fn detect_windows_os_type() -> EOSType {
    // `cmd /c ver` prints e.g. "Microsoft Windows [Version 10.0.19045.4046]".
    // The bracketed part doesn't depend on the locale.
    let version = std::process::Command::new("cmd")
        .args(["/c", "ver"])
        .output()
        .ok()
        .and_then(|output| String::from_utf8(output.stdout).ok())
        .and_then(|text| {
            let start = text.find('[')? + 1;
            let end = text[start..].find(']')? + start;
            Some(text[start..end].trim().to_string())
        })
        // The bracketed text starts with "Version ", e.g.
        // "Version 10.0.26300.9711"; strip it before parsing.
        .and_then(|text| parse_version(strip_version_prefix(&text)));

    let Some((major, minor, build)) = version else {
        // Couldn't get the version; better to report a modern Windows than an
        // unknown platform.
        return EOSType::Windows10;
    };

    match major {
        4 => EOSType::WinNT,
        5 => match minor {
            0 => EOSType::Win2000,
            1 => EOSType::WinXP,
            // Assume nobody runs Windows XP Professional x64 Edition;
            // it is an edition of Windows Server 2003 anyway.
            2 => EOSType::Win2003,
            _ => EOSType::WinUnknown,
        },
        6 => match minor {
            0 => EOSType::WinVista,  // Also Server 2008
            1 => EOSType::Windows7,  // Also Server 2008 R2
            2 => EOSType::Windows8,  // Also Server 2012
            3 => EOSType::Windows81, // Also Server 2012 R2
            _ => EOSType::WinUnknown,
        },
        10 if build >= 22000 => EOSType::Win11,
        10 => EOSType::Windows10, // Also Server 2016, Server 2019, Server 2022
        _ => EOSType::WinUnknown,
    }
}

/// Maps a Linux kernel version (from `/proc/sys/kernel/osrelease`) to Valve's
/// `EOSType`.
#[cfg(target_os = "linux")]
fn detect_linux_os_type() -> EOSType {
    let Some((major, minor, _)) = std::fs::read_to_string("/proc/sys/kernel/osrelease")
        .ok()
        .and_then(|text| parse_version(text.trim()))
    else {
        return EOSType::LinuxUnknown;
    };

    // The specific minor versions only exist in Valve's enum for LTS versions.
    match major {
        2 => match minor {
            2 => EOSType::Linux22,
            4 => EOSType::Linux24,
            6 => EOSType::Linux26,
            _ => EOSType::LinuxUnknown,
        },
        3 => match minor {
            2 => EOSType::Linux32,
            5 => EOSType::Linux35,
            6 => EOSType::Linux36,
            10 => EOSType::Linux310,
            16 => EOSType::Linux316,
            18 => EOSType::Linux318,
            _ => EOSType::Linux3x,
        },
        4 => match minor {
            1 => EOSType::Linux41,
            4 => EOSType::Linux44,
            9 => EOSType::Linux49,
            14 => EOSType::Linux414,
            19 => EOSType::Linux419,
            _ => EOSType::Linux4x,
        },
        5 => match minor {
            4 => EOSType::Linux54,
            10 => EOSType::Linux510,
            _ => EOSType::Linux5x,
        },
        6 => EOSType::Linux6x,
        7 => EOSType::Linux7x,
        _ => EOSType::LinuxUnknown,
    }
}

/// Maps a macOS product version (from `sw_vers`) to Valve's `EOSType`.
#[cfg(target_os = "macos")]
fn detect_macos_os_type() -> EOSType {
    let Some((product_major, product_minor, _)) = std::process::Command::new("sw_vers")
        .arg("-productVersion")
        .output()
        .ok()
        .and_then(|output| String::from_utf8(output.stdout).ok())
        .and_then(|text| parse_version(text.trim()))
    else {
        return EOSType::MacOSUnknown;
    };

    map_macos_version(product_major, product_minor)
}

fn map_macos_version(major: u64, minor: u64) -> EOSType {
    match major {
        10 => match minor {
            11 => EOSType::MacOS1011,
            12 => EOSType::MacOS1012,
            13 => EOSType::Macos1013,
            14 => EOSType::Macos1014,
            16 => EOSType::MacOS1016,
            17 => EOSType::MacOS1017,
            18 => EOSType::MacOS1018,
            19 => EOSType::MacOS1019,
            20 => EOSType::MacOS1020,
            21 => EOSType::MacOS1021,
            22 => EOSType::MacOS1022,
            23 => EOSType::MacOS1023,
            _ => EOSType::Macos1015,
        },
        11 if minor == 0 => EOSType::MacOS11,
        11 => EOSType::MacOS111,
        12 => EOSType::MacOS12,
        13 => EOSType::MacOS13,
        14 => EOSType::MacOS14,
        15 => EOSType::MacOS15,
        26 => EOSType::MacOS26,
        27 => EOSType::MacOS27,
        28 => EOSType::MacOS28,
        _ => EOSType::MacOSUnknown,
    }
}

/// Skips any text before the first digit, e.g. the "Version " in the output of
/// `cmd /c ver`.
fn strip_version_prefix(text: &str) -> &str {
    match text.find(|character: char| character.is_ascii_digit()) {
        Some(start) => &text[start..],
        None => text,
    }
}

/// Parses a dotted version into major, minor and build. Non-numeric trailing
/// parts (like a Linux kernel's `-generic`) are ignored.
fn parse_version(text: &str) -> Option<(u64, u64, u64)> {
    let mut parts = text.split('.');

    let major = parts.next()?.trim().parse::<u64>().ok()?;
    let minor = parts.next().unwrap_or("0").trim().parse::<u64>().ok()?;
    let build = parts.next().unwrap_or("0").trim().parse::<u64>().ok()?;

    Some((major, minor, build))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_macos_product_versions() {
        assert_eq!(EOSType::MacOS1011, map_macos_version(10, 11));
        assert_eq!(EOSType::Macos1015, map_macos_version(10, 15));
        assert_eq!(EOSType::MacOS1016, map_macos_version(10, 16));
        assert_eq!(EOSType::MacOS1021, map_macos_version(10, 21));
        assert_eq!(EOSType::MacOS1023, map_macos_version(10, 23));
        assert_eq!(EOSType::MacOS11, map_macos_version(11, 0));
        assert_eq!(EOSType::MacOS111, map_macos_version(11, 1));
        assert_eq!(EOSType::MacOS15, map_macos_version(15, 0));
        assert_eq!(EOSType::MacOS26, map_macos_version(26, 0));
        assert_eq!(EOSType::MacOS28, map_macos_version(28, 0));
        assert_eq!(EOSType::MacOSUnknown, map_macos_version(99, 0));
    }
}

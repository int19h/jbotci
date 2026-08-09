#[allow(unused_imports)]
use bityzba::{ensures, requires};

/// Matches the wasm stack reserve configured in `.cargo/config.toml`.
pub(crate) const WINDOWS_STACK_RESERVE_BYTES: usize = 8 * 1024 * 1024;

#[requires(true)]
#[ensures((target_os != "windows" -> ret.as_ref().is_ok_and(Option::is_none)) || ret.is_err())]
#[ensures((ret.as_ref().is_ok_and(Option::is_some) -> target_os == "windows") || ret.is_err())]
#[ensures(ret.is_err() -> target_os == "windows")]
pub(crate) fn windows_stack_link_arg(
    target_os: &str,
    target_env: &str,
) -> Result<Option<String>, String> {
    if target_os != "windows" {
        return Ok(None);
    }

    let link_arg = match target_env {
        "msvc" => format!("/STACK:{WINDOWS_STACK_RESERVE_BYTES}"),
        // Rust's MinGW and gnullvm Windows targets both report target_env="gnu"
        // and reach the PE linker through a GNU-style compiler driver.
        "gnu" => format!("-Wl,--stack,{WINDOWS_STACK_RESERVE_BYTES}"),
        _ => {
            return Err(format!(
                "unsupported Windows target environment `{target_env}`; cannot reserve the jbotci CLI stack"
            ));
        }
    };
    Ok(Some(link_arg))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn selects_msvc_pe_stack_reserve() {
        assert_eq!(
            windows_stack_link_arg("windows", "msvc"),
            Ok(Some("/STACK:8388608".to_owned()))
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn selects_gnu_style_pe_stack_reserve_for_mingw_and_gnullvm() {
        // Both target families expose CARGO_CFG_TARGET_ENV=gnu to build.rs.
        assert_eq!(
            windows_stack_link_arg("windows", "gnu"),
            Ok(Some("-Wl,--stack,8388608".to_owned()))
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn omits_pe_stack_reserve_for_non_windows_targets() {
        assert_eq!(windows_stack_link_arg("linux", "gnu"), Ok(None));
        assert_eq!(windows_stack_link_arg("macos", ""), Ok(None));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn rejects_unknown_windows_target_environment() {
        let error = windows_stack_link_arg("windows", "future-env")
            .expect_err("unknown Windows target environment must fail");
        assert!(error.contains("future-env"));
    }
}

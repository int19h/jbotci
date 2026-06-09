use std::collections::BTreeSet;
use std::ffi::{OsStr, OsString};
use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::process::{Command as ProcessCommand, ExitStatus, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};

use anyhow::{Context, Result, bail};
use bityzba::*;
use clap::{Args, Parser, Subcommand, ValueEnum};
use sha2::{Digest, Sha256};
use walkdir::WalkDir;

const DEFAULT_TEST_JOBS_TEXT: &str = "16";
const DIOXUS_WEB_RELEASE_DIR: &str = "target/dx/jbotci-app/release/web";
const DIOXUS_DESKTOP_BUNDLE_DIR: &str = "target/dx/jbotci-app/bundle";
const DIOXUS_WEB_PUBLIC_INPUT_DIR: &str = "target/jbotci-web-public";
const DIOXUS_DESKTOP_DEV_PROFILE: &str = "desktop-dev";
const DESKTOP_BUNDLE_OUT_DIR: &str = "target/jbotci-desktop-bundles";
const DESKTOP_BUNDLE_RAW_DIR_NAME: &str = "raw";
const DESKTOP_BUNDLE_MACOS_ARTIFACT: &str = "jbotci.app";
const DESKTOP_BUNDLE_LINUX_ARTIFACT: &str = "jbotci.appimage";
const DESKTOP_BUNDLE_WINDOWS_ARTIFACT: &str = "jbotci.msi";
const SHARED_UI_ASSET_DIR: &str = "crates/jbotci-ui/assets";
const RELEASE_SERVICE_WORKER_FILE_NAME: &str = "service-worker.js";
const WEB_ASSET_SYNC_TEMP_DIR: &str = "target/jbotci-web-public-sync";
static WEB_ASSET_COPY_COUNTER: AtomicUsize = AtomicUsize::new(0);

#[derive(Debug, Parser)]
#[command(name = "xtask")]
#[command(about = "Workspace automation for jbotci")]
#[invariant(true)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
#[invariant(true)]
#[invariant(::Fmt => true)]
#[invariant(::DesktopBuild => true)]
#[invariant(::DesktopServe => true)]
#[invariant(::DesktopBundleMacos => true)]
#[invariant(::DesktopBundleLinux => true)]
#[invariant(::DesktopBundleWindows => true)]
#[invariant(::DistServer(..) => true)]
#[invariant(::RenderDockerBuild(..) => true)]
#[invariant(::RenderDockerRun(..) => true)]
enum Command {
    Check,
    Test,
    Clippy,
    Fmt {
        #[arg(long)]
        check: bool,
    },
    DesktopBuild,
    DesktopServe,
    DesktopBundleMacos,
    DesktopBundleLinux,
    DesktopBundleWindows,
    DistServer(DistServerArgs),
    RenderDockerBuild(RenderDockerBuildArgs),
    RenderDockerRun(RenderDockerRunArgs),
}

#[derive(Debug, Args)]
#[invariant(true)]
struct DistServerArgs {
    #[arg(long, default_value = ".jbotci-build/jbotci-web")]
    out_dir: PathBuf,
    #[arg(long, default_value = "/")]
    base_path: String,
    #[arg(long)]
    skip_web_bundle: bool,
    #[arg(long)]
    skip_web_embeddings: bool,
    #[arg(long = "embedding-dtype", default_values_t = ["q4".to_owned(), "q8".to_owned()])]
    embedding_dtypes: Vec<String>,
    #[arg(long, default_value = "transformers")]
    embedding_backend: String,
}

#[derive(Debug, Args)]
#[invariant(true)]
struct RenderDockerBuildArgs {
    #[arg(long, value_enum, default_value = "auto")]
    engine: ContainerEngineArg,
    #[arg(long, default_value = "jbotci-render:local")]
    image: String,
    #[arg(long, default_value = "/")]
    base_path: String,
    #[arg(long, default_value = "https://assets.jbotci.app/embeddings/web/v1")]
    web_embeddings_base_url: String,
    #[arg(long)]
    no_cache: bool,
}

#[derive(Debug, Args)]
#[invariant(true)]
struct RenderDockerRunArgs {
    #[arg(long, value_enum, default_value = "auto")]
    engine: ContainerEngineArg,
    #[arg(long, default_value = "jbotci-render:local")]
    image: String,
    #[arg(long, default_value_t = 8080)]
    host_port: u16,
    #[arg(long, default_value_t = 10000)]
    container_port: u16,
    #[arg(long, default_value = "/")]
    base_path: String,
    #[arg(long, default_value = "https://assets.jbotci.app/embeddings/web/v1")]
    web_embeddings_base_url: String,
    #[arg(long)]
    no_build: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
#[invariant(true)]
enum ContainerEngineArg {
    Auto,
    Docker,
    Podman,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[invariant(true)]
enum ContainerEngine {
    Docker,
    Podman,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[invariant(true)]
enum DesktopBundleTarget {
    Macos,
    Linux,
    Windows,
}

impl ContainerEngineArg {
    #[requires(true)]
    #[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
    fn resolve(self) -> Result<ContainerEngine> {
        match self {
            Self::Docker => Ok(ContainerEngine::Docker),
            Self::Podman => Ok(ContainerEngine::Podman),
            Self::Auto => {
                if container_engine_available(ContainerEngine::Docker.command_name()) {
                    Ok(ContainerEngine::Docker)
                } else if container_engine_available(ContainerEngine::Podman.command_name()) {
                    Ok(ContainerEngine::Podman)
                } else {
                    bail!("could not find `docker` or `podman` in PATH")
                }
            }
        }
    }
}

impl ContainerEngine {
    #[requires(true)]
    #[ensures(!ret.is_empty())]
    fn command_name(self) -> &'static str {
        match self {
            Self::Docker => "docker",
            Self::Podman => "podman",
        }
    }
}

impl DesktopBundleTarget {
    #[requires(true)]
    #[ensures(!ret.is_empty())]
    fn dx_platform_flag(self) -> &'static str {
        match self {
            Self::Macos => "--macos",
            Self::Linux => "--linux",
            Self::Windows => "--windows",
        }
    }

    #[requires(true)]
    #[ensures(!ret.is_empty())]
    fn dx_package_type(self) -> &'static str {
        match self {
            Self::Macos => "macos",
            Self::Linux => "appimage",
            Self::Windows => "msi",
        }
    }

    #[requires(true)]
    #[ensures(!ret.is_empty())]
    fn platform_dir_name(self) -> &'static str {
        match self {
            Self::Macos => "macos",
            Self::Linux => "linux",
            Self::Windows => "windows",
        }
    }

    #[requires(true)]
    #[ensures(!ret.is_empty())]
    fn public_artifact_name(self) -> &'static str {
        match self {
            Self::Macos => DESKTOP_BUNDLE_MACOS_ARTIFACT,
            Self::Linux => DESKTOP_BUNDLE_LINUX_ARTIFACT,
            Self::Windows => DESKTOP_BUNDLE_WINDOWS_ARTIFACT,
        }
    }

    #[requires(true)]
    #[ensures(!ret.is_empty())]
    fn raw_artifact_suffix(self) -> &'static str {
        match self {
            Self::Macos => ".app",
            Self::Linux => ".appimage",
            Self::Windows => ".msi",
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn raw_artifact_is_directory(self) -> bool {
        matches!(self, Self::Macos)
    }

    #[requires(true)]
    #[ensures(!ret.is_empty())]
    fn dx_command_description(self, raw_dir: &Path) -> String {
        format!(
            "dx bundle {} -p jbotci-app --release --package-types {} --out-dir {}",
            self.dx_platform_flag(),
            self.dx_package_type(),
            raw_dir.display()
        )
    }
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn main() -> Result<()> {
    let args = std::env::args_os().collect::<Vec<_>>();
    if should_run_light_command(&args) {
        run_light_command(args)
    } else {
        delegate_to_xtask_full(&args)
    }
}

#[requires(!args.is_empty())]
#[ensures(true)]
fn should_run_light_command(args: &[OsString]) -> bool {
    match first_subcommand(args) {
        Some(
            "check"
            | "test"
            | "clippy"
            | "fmt"
            | "desktop-build"
            | "desktop-serve"
            | "desktop-bundle-macos"
            | "desktop-bundle-linux"
            | "desktop-bundle-windows"
            | "render-docker-build"
            | "render-docker-run",
        ) => true,
        Some("dist-server") => dist_server_args_request_light_path(args),
        _ => false,
    }
}

#[requires(!args.is_empty())]
#[ensures(true)]
fn first_subcommand(args: &[OsString]) -> Option<&str> {
    let command = args.get(1)?.to_str()?;
    if command.starts_with('-') {
        None
    } else {
        Some(command)
    }
}

#[requires(!args.is_empty())]
#[ensures(true)]
fn dist_server_args_request_light_path(args: &[OsString]) -> bool {
    let has_skip_embeddings = args
        .iter()
        .skip(2)
        .any(|arg| arg == OsStr::new("--skip-web-embeddings"));
    let has_skip_bundle = args
        .iter()
        .skip(2)
        .any(|arg| arg == OsStr::new("--skip-web-bundle"));
    has_skip_embeddings && !has_skip_bundle
}

#[requires(!args.is_empty())]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn run_light_command(args: Vec<OsString>) -> Result<()> {
    let cli = Cli::parse_from(args);
    match cli.command {
        Command::Check => cargo(&[
            "check",
            "--workspace",
            "--all-targets",
            "-j",
            DEFAULT_TEST_JOBS_TEXT,
        ]),
        Command::Test => cargo(&[
            "test",
            "--workspace",
            "--all-targets",
            "-j",
            DEFAULT_TEST_JOBS_TEXT,
            "--",
            "--test-threads",
            DEFAULT_TEST_JOBS_TEXT,
        ]),
        Command::Clippy => cargo(&[
            "clippy",
            "--workspace",
            "--all-targets",
            "-j",
            DEFAULT_TEST_JOBS_TEXT,
            "--",
            "-D",
            "warnings",
        ]),
        Command::Fmt { check } => {
            if check {
                cargo(&["fmt", "--all", "--", "--check"])
            } else {
                cargo(&["fmt", "--all"])
            }
        }
        Command::DesktopBuild => dx_desktop_build(),
        Command::DesktopServe => dx_desktop_serve(),
        Command::DesktopBundleMacos => dx_desktop_bundle(DesktopBundleTarget::Macos),
        Command::DesktopBundleLinux => dx_desktop_bundle(DesktopBundleTarget::Linux),
        Command::DesktopBundleWindows => dx_desktop_bundle(DesktopBundleTarget::Windows),
        Command::DistServer(args) => dist_server(args),
        Command::RenderDockerBuild(args) => render_docker_build(args),
        Command::RenderDockerRun(args) => render_docker_run(args),
    }
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn dx_desktop_build() -> Result<()> {
    let status = ProcessCommand::new("dx")
        .arg("build")
        .arg("--desktop")
        .arg("-p")
        .arg("jbotci-app")
        .arg("--profile")
        .arg(DIOXUS_DESKTOP_DEV_PROFILE)
        .status()
        .context("failed to run `dx build --desktop -p jbotci-app --profile desktop-dev`")?;
    check_status(
        status,
        "dx build --desktop -p jbotci-app --profile desktop-dev",
    )
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn dx_desktop_serve() -> Result<()> {
    let status = ProcessCommand::new("dx")
        .arg("serve")
        .arg("--desktop")
        .arg("-p")
        .arg("jbotci-app")
        .arg("--profile")
        .arg(DIOXUS_DESKTOP_DEV_PROFILE)
        .status()
        .context("failed to run `dx serve --desktop -p jbotci-app --profile desktop-dev`")?;
    check_status(
        status,
        "dx serve --desktop -p jbotci-app --profile desktop-dev",
    )
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn dx_desktop_bundle(target: DesktopBundleTarget) -> Result<()> {
    let output_dir = desktop_bundle_output_dir(target)?;
    let raw_dir = output_dir.join(DESKTOP_BUNDLE_RAW_DIR_NAME);
    clean_dioxus_desktop_bundle_staging(target)?;
    if raw_dir.exists() {
        fs::remove_dir_all(&raw_dir).with_context(|| {
            format!(
                "removing old desktop bundle raw dir `{}`",
                raw_dir.display()
            )
        })?;
    }
    fs::create_dir_all(&raw_dir)
        .with_context(|| format!("creating desktop bundle raw dir `{}`", raw_dir.display()))?;
    let command_text = target.dx_command_description(&raw_dir);
    let status = ProcessCommand::new("dx")
        .arg("bundle")
        .arg(target.dx_platform_flag())
        .arg("-p")
        .arg("jbotci-app")
        .arg("--release")
        .arg("--package-types")
        .arg(target.dx_package_type())
        .arg("--out-dir")
        .arg(&raw_dir)
        .status()
        .with_context(|| format!("failed to run `{command_text}`"))?;
    check_status(status, &command_text)?;
    let published = publish_desktop_bundle_artifact(target, &raw_dir, &output_dir)?;
    println!("published desktop bundle `{}`", published.display());
    Ok(())
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn clean_dioxus_desktop_bundle_staging(target: DesktopBundleTarget) -> Result<()> {
    let staging_dir = Path::new(DIOXUS_DESKTOP_BUNDLE_DIR).join(target.platform_dir_name());
    match fs::remove_dir_all(&staging_dir) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error).with_context(|| {
            format!(
                "removing Dioxus desktop bundle staging dir `{}`",
                staging_dir.display()
            )
        }),
    }
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|path| path.ends_with(target.platform_dir_name())) || ret.is_err())]
fn desktop_bundle_output_dir(target: DesktopBundleTarget) -> Result<PathBuf> {
    absolute_path(&Path::new(DESKTOP_BUNDLE_OUT_DIR).join(target.platform_dir_name()))
}

#[requires(raw_dir.is_dir())]
#[requires(output_dir.is_dir())]
#[ensures(ret.as_ref().is_ok_and(|path| path.file_name().is_some_and(|name| name == OsStr::new(target.public_artifact_name()))) || ret.is_err())]
fn publish_desktop_bundle_artifact(
    target: DesktopBundleTarget,
    raw_dir: &Path,
    output_dir: &Path,
) -> Result<PathBuf> {
    let raw_artifact = find_desktop_bundle_raw_artifact(target, raw_dir)?;
    let public_artifact = desktop_bundle_public_path(target, output_dir);
    copy_desktop_bundle_artifact(target, &raw_artifact, &public_artifact)?;
    Ok(public_artifact)
}

#[requires(true)]
#[ensures(ret.file_name().is_some_and(|name| name == OsStr::new(target.public_artifact_name())))]
fn desktop_bundle_public_path(target: DesktopBundleTarget, output_dir: &Path) -> PathBuf {
    output_dir.join(target.public_artifact_name())
}

#[requires(raw_dir.is_dir())]
#[ensures(ret.as_ref().is_ok_and(|path| path.exists()) || ret.is_err())]
fn find_desktop_bundle_raw_artifact(
    target: DesktopBundleTarget,
    raw_dir: &Path,
) -> Result<PathBuf> {
    let mut matches = Vec::new();
    for entry in WalkDir::new(raw_dir).min_depth(1) {
        let entry = entry
            .with_context(|| format!("walking desktop bundle raw dir `{}`", raw_dir.display()))?;
        if desktop_bundle_entry_matches(target, &entry) {
            matches.push(entry.path().to_path_buf());
        }
    }
    match matches.len() {
        1 => Ok(matches.remove(0)),
        0 => bail!(
            "could not find a {} artifact under `{}`",
            target.raw_artifact_suffix(),
            raw_dir.display()
        ),
        count => bail!(
            "found {count} {} artifacts under `{}`, expected exactly one",
            target.raw_artifact_suffix(),
            raw_dir.display()
        ),
    }
}

#[requires(true)]
#[ensures(true)]
fn desktop_bundle_entry_matches(target: DesktopBundleTarget, entry: &walkdir::DirEntry) -> bool {
    let file_type = entry.file_type();
    if target.raw_artifact_is_directory() {
        if !file_type.is_dir() {
            return false;
        }
    } else if !file_type.is_file() {
        return false;
    }
    entry.file_name().to_str().is_some_and(|name| {
        name.to_ascii_lowercase()
            .ends_with(target.raw_artifact_suffix())
    })
}

#[requires(source.exists())]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn copy_desktop_bundle_artifact(
    target: DesktopBundleTarget,
    source: &Path,
    destination: &Path,
) -> Result<()> {
    remove_existing_path(destination)?;
    if target.raw_artifact_is_directory() {
        copy_directory_recursively(source, destination)
    } else {
        copy_file_artifact(source, destination)
    }
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn remove_existing_path(path: &Path) -> Result<()> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(()),
        Err(error) => {
            return Err(error)
                .with_context(|| format!("reading existing path `{}`", path.display()));
        }
    };
    if metadata.file_type().is_dir() && !metadata.file_type().is_symlink() {
        fs::remove_dir_all(path)
            .with_context(|| format!("removing existing directory `{}`", path.display()))
    } else {
        fs::remove_file(path)
            .with_context(|| format!("removing existing file `{}`", path.display()))
    }
}

#[requires(source.is_file())]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn copy_file_artifact(source: &Path, destination: &Path) -> Result<()> {
    let parent = destination.parent().with_context(|| {
        format!(
            "desktop bundle output `{}` has no parent",
            destination.display()
        )
    })?;
    fs::create_dir_all(parent)
        .with_context(|| format!("creating desktop bundle output dir `{}`", parent.display()))?;
    fs::copy(source, destination).with_context(|| {
        format!(
            "copying desktop bundle artifact `{}` to `{}`",
            source.display(),
            destination.display()
        )
    })?;
    let permissions = fs::metadata(source)
        .with_context(|| format!("reading permissions for `{}`", source.display()))?
        .permissions();
    fs::set_permissions(destination, permissions)
        .with_context(|| format!("setting permissions on `{}`", destination.display()))
}

#[requires(source.is_dir())]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn copy_directory_recursively(source: &Path, destination: &Path) -> Result<()> {
    fs::create_dir_all(destination).with_context(|| {
        format!(
            "creating desktop bundle directory `{}`",
            destination.display()
        )
    })?;
    for entry in WalkDir::new(source).min_depth(1) {
        let entry = entry
            .with_context(|| format!("walking desktop bundle directory `{}`", source.display()))?;
        let relative = entry.path().strip_prefix(source).with_context(|| {
            format!(
                "making `{}` relative to `{}`",
                entry.path().display(),
                source.display()
            )
        })?;
        let target = destination.join(relative);
        if entry.file_type().is_dir() {
            fs::create_dir_all(&target).with_context(|| {
                format!("creating desktop bundle directory `{}`", target.display())
            })?;
        } else if entry.file_type().is_file() {
            copy_file_artifact(entry.path(), &target)?;
        } else if entry.file_type().is_symlink() {
            copy_symlink(entry.path(), &target)?;
        }
    }
    Ok(())
}

#[cfg(unix)]
#[requires(source.exists())]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn copy_symlink(source: &Path, destination: &Path) -> Result<()> {
    let parent = destination.parent().with_context(|| {
        format!(
            "desktop bundle symlink `{}` has no parent",
            destination.display()
        )
    })?;
    fs::create_dir_all(parent)
        .with_context(|| format!("creating desktop bundle symlink dir `{}`", parent.display()))?;
    let link_target =
        fs::read_link(source).with_context(|| format!("reading symlink `{}`", source.display()))?;
    std::os::unix::fs::symlink(&link_target, destination).with_context(|| {
        format!(
            "copying symlink `{}` to `{}`",
            source.display(),
            destination.display()
        )
    })
}

#[cfg(windows)]
#[requires(source.exists())]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn copy_symlink(source: &Path, destination: &Path) -> Result<()> {
    let parent = destination.parent().with_context(|| {
        format!(
            "desktop bundle symlink `{}` has no parent",
            destination.display()
        )
    })?;
    fs::create_dir_all(parent)
        .with_context(|| format!("creating desktop bundle symlink dir `{}`", parent.display()))?;
    let link_target =
        fs::read_link(source).with_context(|| format!("reading symlink `{}`", source.display()))?;
    let metadata = fs::metadata(source)
        .with_context(|| format!("reading symlink target metadata `{}`", source.display()))?;
    if metadata.is_dir() {
        std::os::windows::fs::symlink_dir(&link_target, destination)
    } else {
        std::os::windows::fs::symlink_file(&link_target, destination)
    }
    .with_context(|| {
        format!(
            "copying symlink `{}` to `{}`",
            source.display(),
            destination.display()
        )
    })
}

#[requires(!args.is_empty())]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn delegate_to_xtask_full(args: &[OsString]) -> Result<()> {
    let cargo = std::env::var_os("CARGO").unwrap_or_else(|| OsString::from("cargo"));
    let mut command = ProcessCommand::new(cargo);
    command.arg("run").arg("-p").arg("xtask-full").arg("--");
    command.args(args.iter().skip(1));
    let status = command
        .status()
        .context("failed to delegate to `xtask-full`")?;
    check_status(status, "cargo run -p xtask-full")
}

#[requires(!args.is_empty(), "cargo subcommand arguments must not be empty")]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn cargo(args: &[&str]) -> Result<()> {
    let status = ProcessCommand::new("cargo")
        .args(args)
        .status()
        .with_context(|| format!("failed to run `cargo {}`", args.join(" ")))?;
    check_status(status, &format!("cargo {}", args.join(" ")))
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn dist_server(args: DistServerArgs) -> Result<()> {
    if args.skip_web_bundle {
        bail!("lightweight `dist-server` cannot skip the web bundle");
    }
    if !args.skip_web_embeddings {
        bail!("lightweight `dist-server` requires `--skip-web-embeddings`");
    }
    let _ignored_embedding_options = (&args.embedding_dtypes, &args.embedding_backend);
    let out_dir = absolute_path(&args.out_dir)?;
    if out_dir.exists() {
        fs::remove_dir_all(&out_dir)
            .with_context(|| format!("removing old web bundle `{}`", out_dir.display()))?;
    }
    run_dx_bundle(&out_dir, &args.base_path)?;
    server_bundle_path(&out_dir).map(|_| ())
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn run_dx_bundle(out_dir: &Path, base_path: &str) -> Result<()> {
    clean_dioxus_web_release_output()?;
    prepare_dioxus_web_public_input()?;
    let mut command = ProcessCommand::new("dx");
    set_dioxus_base_path_env(&mut command, base_path);
    command
        .arg("bundle")
        .arg("--out-dir")
        .arg(out_dir)
        .arg("@client")
        .arg("--web")
        .arg("-p")
        .arg("jbotci-app")
        .arg("--release")
        .arg("--debug-symbols=false")
        .arg("--inject-loading-scripts=false")
        .arg("--base-path")
        .arg(base_path)
        .arg("@server")
        .arg("--server")
        .arg("-p")
        .arg("jbotci-server")
        .arg("--release");
    let status = command.status().context("failed to run `dx bundle`")?;
    check_status(
        status,
        "dx bundle @client --web -p jbotci-app --release @server --server -p jbotci-server --release",
    )?;
    let web_dist = web_dist_dir(out_dir)?;
    write_release_service_worker(&web_dist)?;
    server_bundle_path(out_dir)?;
    Ok(())
}

#[requires(true)]
#[ensures(true)]
fn set_dioxus_base_path_env(command: &mut ProcessCommand, base_path: &str) {
    if let Some(asset_root) = dioxus_asset_root(base_path) {
        command.env("DIOXUS_ASSET_ROOT", asset_root);
    } else {
        command.env_remove("DIOXUS_ASSET_ROOT");
    }
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|root| root.starts_with('/') && root.len() > 1))]
#[ensures(ret.as_ref().is_none_or(|root| !root.ends_with('/')))]
fn dioxus_asset_root(base_path: &str) -> Option<String> {
    let trimmed = base_path.trim().trim_matches('/');
    if trimmed.is_empty() {
        None
    } else {
        Some(format!("/{trimmed}"))
    }
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn clean_dioxus_web_release_output() -> Result<()> {
    let release_dir = Path::new(DIOXUS_WEB_RELEASE_DIR);
    if release_dir.exists() {
        fs::remove_dir_all(release_dir).with_context(|| {
            format!(
                "removing old Dioxus release web output `{}`",
                release_dir.display()
            )
        })?;
    }
    Ok(())
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn prepare_dioxus_web_public_input() -> Result<()> {
    remove_obsolete_web_public_assets(&dioxus_web_public_input_dir())?;
    copy_stable_web_assets_to_public(&dioxus_web_public_input_dir())
}

#[requires(true)]
#[ensures(!ret.as_os_str().is_empty())]
fn dioxus_web_public_input_dir() -> PathBuf {
    PathBuf::from(DIOXUS_WEB_PUBLIC_INPUT_DIR)
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn remove_obsolete_web_public_assets(public_dir: &Path) -> Result<()> {
    remove_obsolete_web_public_dir(public_dir, Path::new("assets/generated"))?;
    remove_obsolete_web_public_file(public_dir, Path::new("manifest.webmanifest"))?;
    remove_obsolete_web_public_file(public_dir, Path::new("assets/manifest.webmanifest"))
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn remove_obsolete_web_public_dir(public_dir: &Path, relative: &Path) -> Result<()> {
    let path = public_dir.join(relative);
    match fs::remove_dir_all(&path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error).with_context(|| {
            format!(
                "removing obsolete web public asset directory `{}`",
                path.display()
            )
        }),
    }
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn remove_obsolete_web_public_file(public_dir: &Path, relative: &Path) -> Result<()> {
    let path = public_dir.join(relative);
    match fs::remove_file(&path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error).with_context(|| {
            format!(
                "removing obsolete web public asset file `{}`",
                path.display()
            )
        }),
    }
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn copy_stable_web_assets_to_public(public_dir: &Path) -> Result<()> {
    copy_stable_web_asset_file(
        public_dir,
        Path::new("manifest.webmanifest"),
        Path::new("assets/manifest.webmanifest"),
    )?;
    copy_stable_web_asset_dir(public_dir, Path::new("icons"), Path::new("assets/icons"))?;
    copy_stable_web_asset_dir(
        public_dir,
        Path::new("cll/media"),
        Path::new("assets/cll/media"),
    )
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn copy_stable_web_asset_file(
    public_dir: &Path,
    source_relative: &Path,
    target_relative: &Path,
) -> Result<()> {
    let source = Path::new(SHARED_UI_ASSET_DIR).join(source_relative);
    let target = public_dir.join(target_relative);
    copy_web_asset_file_atomically(&source, &target, "stable web asset")
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn copy_stable_web_asset_dir(
    public_dir: &Path,
    source_relative: &Path,
    target_relative: &Path,
) -> Result<()> {
    let source_dir = Path::new(SHARED_UI_ASSET_DIR).join(source_relative);
    let target_dir = public_dir.join(target_relative);
    copy_flat_web_asset_dir(&source_dir, &target_dir, "stable web asset")
}

#[requires(source_dir.is_dir())]
#[requires(!description.is_empty())]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn copy_flat_web_asset_dir(source_dir: &Path, target_dir: &Path, description: &str) -> Result<()> {
    fs::create_dir_all(target_dir).with_context(|| {
        format!(
            "creating {description} directory `{}`",
            target_dir.display()
        )
    })?;
    let mut source_file_names = BTreeSet::new();
    for entry in fs::read_dir(source_dir)
        .with_context(|| format!("reading {description}s from `{}`", source_dir.display()))?
    {
        let entry = entry
            .with_context(|| format!("reading {description} under `{}`", source_dir.display()))?;
        if !entry
            .file_type()
            .with_context(|| format!("reading file type for `{}`", entry.path().display()))?
            .is_file()
        {
            continue;
        }
        let file_name = entry.file_name();
        source_file_names.insert(file_name.clone());
        let target = target_dir.join(file_name);
        copy_web_asset_file_atomically(&entry.path(), &target, description)?;
    }
    remove_obsolete_flat_web_asset_files(target_dir, &source_file_names, description)
}

#[requires(source.is_file())]
#[requires(!description.is_empty())]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn copy_web_asset_file_atomically(source: &Path, target: &Path, description: &str) -> Result<()> {
    let parent = target
        .parent()
        .with_context(|| format!("{description} target `{}` has no parent", target.display()))?;
    fs::create_dir_all(parent)
        .with_context(|| format!("creating {description} directory `{}`", parent.display()))?;
    let temp_dir = web_asset_sync_temp_dir(target);
    fs::create_dir_all(&temp_dir).with_context(|| {
        format!(
            "creating temporary {description} directory `{}`",
            temp_dir.display()
        )
    })?;
    let file_name = target.file_name().with_context(|| {
        format!(
            "{description} target `{}` has no file name",
            target.display()
        )
    })?;
    let temp_path = temp_dir.join(format!(
        "{}-{}-{}.tmp",
        std::process::id(),
        WEB_ASSET_COPY_COUNTER.fetch_add(1, Ordering::Relaxed),
        file_name.to_string_lossy()
    ));
    fs::copy(source, &temp_path).with_context(|| {
        format!(
            "copying {description} `{}` to temporary file `{}`",
            source.display(),
            temp_path.display()
        )
    })?;
    match fs::rename(&temp_path, target) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == ErrorKind::AlreadyExists => {
            fs::remove_file(target).with_context(|| {
                format!(
                    "removing old {description} `{}` before replace",
                    target.display()
                )
            })?;
            fs::rename(&temp_path, target).with_context(|| {
                format!(
                    "moving temporary {description} `{}` to `{}`",
                    temp_path.display(),
                    target.display()
                )
            })
        }
        Err(error) => {
            let _ = fs::remove_file(&temp_path);
            Err(error).with_context(|| {
                format!(
                    "moving temporary {description} `{}` to `{}`",
                    temp_path.display(),
                    target.display()
                )
            })
        }
    }
}

#[requires(!contents.is_empty())]
#[requires(!description.is_empty())]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn write_web_asset_text_atomically(target: &Path, contents: &str, description: &str) -> Result<()> {
    let parent = target
        .parent()
        .with_context(|| format!("{description} target `{}` has no parent", target.display()))?;
    fs::create_dir_all(parent)
        .with_context(|| format!("creating {description} directory `{}`", parent.display()))?;
    let temp_dir = web_asset_sync_temp_dir(target);
    fs::create_dir_all(&temp_dir).with_context(|| {
        format!(
            "creating temporary {description} directory `{}`",
            temp_dir.display()
        )
    })?;
    let file_name = target.file_name().with_context(|| {
        format!(
            "{description} target `{}` has no file name",
            target.display()
        )
    })?;
    let temp_path = temp_dir.join(format!(
        "{}-{}-{}.tmp",
        std::process::id(),
        WEB_ASSET_COPY_COUNTER.fetch_add(1, Ordering::Relaxed),
        file_name.to_string_lossy()
    ));
    fs::write(&temp_path, contents).with_context(|| {
        format!(
            "writing {description} temporary file `{}`",
            temp_path.display()
        )
    })?;
    match fs::rename(&temp_path, target) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == ErrorKind::AlreadyExists => {
            fs::remove_file(target).with_context(|| {
                format!(
                    "removing old {description} `{}` before replace",
                    target.display()
                )
            })?;
            fs::rename(&temp_path, target).with_context(|| {
                format!(
                    "moving temporary {description} `{}` to `{}`",
                    temp_path.display(),
                    target.display()
                )
            })
        }
        Err(error) => {
            let _ = fs::remove_file(&temp_path);
            Err(error).with_context(|| {
                format!(
                    "moving temporary {description} `{}` to `{}`",
                    temp_path.display(),
                    target.display()
                )
            })
        }
    }
}

#[requires(true)]
#[ensures(!ret.as_os_str().is_empty())]
fn web_asset_sync_temp_dir(target: &Path) -> PathBuf {
    let public_input_dir = dioxus_web_public_input_dir();
    if target.starts_with(&public_input_dir) {
        return PathBuf::from(WEB_ASSET_SYNC_TEMP_DIR);
    }
    target
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join(".jbotci-asset-sync")
}

#[requires(!description.is_empty())]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn remove_obsolete_flat_web_asset_files(
    target_dir: &Path,
    source_file_names: &BTreeSet<OsString>,
    description: &str,
) -> Result<()> {
    let entries = match fs::read_dir(target_dir) {
        Ok(entries) => entries,
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(()),
        Err(error) => {
            return Err(error).with_context(|| {
                format!(
                    "reading {description} target directory `{}`",
                    target_dir.display()
                )
            });
        }
    };
    for entry in entries {
        let entry = entry
            .with_context(|| format!("reading {description} under `{}`", target_dir.display()))?;
        if source_file_names.contains(&entry.file_name()) {
            continue;
        }
        let file_type = match entry.file_type() {
            Ok(file_type) => file_type,
            Err(error) if error.kind() == ErrorKind::NotFound => continue,
            Err(error) => {
                return Err(error).with_context(|| {
                    format!("reading file type for `{}`", entry.path().display())
                });
            }
        };
        if file_type.is_file() || file_type.is_symlink() {
            match fs::remove_file(entry.path()) {
                Ok(()) => {}
                Err(error) if error.kind() == ErrorKind::NotFound => {}
                Err(error) => {
                    return Err(error).with_context(|| {
                        format!(
                            "removing obsolete {description} `{}`",
                            entry.path().display()
                        )
                    });
                }
            }
        }
    }
    Ok(())
}

#[requires(out_dir.is_absolute())]
#[ensures(ret.as_ref().is_ok_and(|path| path.is_dir()) || ret.is_err())]
fn web_dist_dir(out_dir: &Path) -> Result<PathBuf> {
    let candidates = [out_dir.join("public"), out_dir.to_path_buf()];
    candidates
        .into_iter()
        .find(|candidate| candidate.join("index.html").is_file())
        .ok_or_else(|| {
            anyhow::anyhow!(
                "could not find Dioxus web `index.html` under `{}`",
                out_dir.display()
            )
        })
}

#[requires(out_dir.is_absolute())]
#[ensures(ret.as_ref().is_ok_and(|path| path.is_file()) || ret.is_err())]
fn server_bundle_path(out_dir: &Path) -> Result<PathBuf> {
    let server = out_dir.join("server");
    if server.is_file() {
        Ok(server)
    } else {
        bail!(
            "could not find Dioxus server bundle executable at `{}`",
            server.display()
        )
    }
}

#[requires(!args.image.trim().is_empty())]
#[requires(!args.web_embeddings_base_url.trim().is_empty())]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn render_docker_build(args: RenderDockerBuildArgs) -> Result<()> {
    let engine = args.engine.resolve()?;
    let engine_command = engine.command_name();
    let git_commit = current_git_commit()?;
    let mut command = ProcessCommand::new(engine_command);
    command.arg("build");
    if args.no_cache {
        command.arg("--no-cache");
    }
    command
        .arg("-f")
        .arg("deploy/render/Dockerfile")
        .arg("-t")
        .arg(&args.image)
        .arg("--build-arg")
        .arg(format!("BASE_PATH={}", args.base_path))
        .arg("--build-arg")
        .arg(format!(
            "WEB_EMBEDDINGS_BASE_URL={}",
            args.web_embeddings_base_url
        ))
        .arg("--build-arg")
        .arg(format!("RENDER_GIT_COMMIT={git_commit}"))
        .arg(".");
    let status = command
        .status()
        .context("failed to run Render Docker build")?;
    check_status(
        status,
        &format!("{engine_command} build -f deploy/render/Dockerfile"),
    )
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|commit| is_git_commit_hash(commit)) || ret.is_err())]
fn current_git_commit() -> Result<String> {
    let output = ProcessCommand::new("git")
        .arg("rev-parse")
        .arg("HEAD")
        .output()
        .context("failed to run `git rev-parse HEAD` for Render Docker build")?;
    if !output.status.success() {
        bail!("`git rev-parse HEAD` failed while preparing Render Docker build");
    }
    let commit = String::from_utf8(output.stdout)
        .context("`git rev-parse HEAD` did not return UTF-8 output")?
        .trim()
        .to_owned();
    if !is_git_commit_hash(&commit) {
        bail!(
            "`git rev-parse HEAD` returned `{commit}`, expected a 40-character hexadecimal Git commit hash"
        );
    }
    Ok(commit)
}

#[requires(true)]
#[ensures(true)]
fn is_git_commit_hash(value: &str) -> bool {
    value.len() == 40 && value.chars().all(|character| character.is_ascii_hexdigit())
}

#[requires(!args.image.trim().is_empty())]
#[requires(!args.web_embeddings_base_url.trim().is_empty())]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn render_docker_run(args: RenderDockerRunArgs) -> Result<()> {
    if !args.no_build {
        render_docker_build(RenderDockerBuildArgs {
            engine: args.engine,
            image: args.image.clone(),
            base_path: args.base_path.clone(),
            web_embeddings_base_url: args.web_embeddings_base_url.clone(),
            no_cache: false,
        })?;
    }
    let engine = args.engine.resolve()?;
    let engine_command = engine.command_name();
    let host_url = format!("http://127.0.0.1:{}", args.host_port);
    println!("running {} on {}", args.image, host_url);
    let status = ProcessCommand::new(engine_command)
        .arg("run")
        .arg("--rm")
        .arg("-p")
        .arg(format!(
            "127.0.0.1:{}:{}",
            args.host_port, args.container_port
        ))
        .arg("-e")
        .arg("IP=0.0.0.0")
        .arg("-e")
        .arg(format!("PORT={}", args.container_port))
        .arg("-e")
        .arg(format!(
            "DIOXUS_ASSET_ROOT={}",
            dioxus_runtime_asset_root(&args.base_path)
        ))
        .arg("-e")
        .arg("DIOXUS_PUBLIC_PATH=/opt/jbotci/public")
        .arg("-e")
        .arg(format!(
            "JBOTCI_WEB_EMBEDDINGS_BASE_URL={}",
            args.web_embeddings_base_url
        ))
        .arg(&args.image)
        .status()
        .context("failed to run Render Docker image")?;
    check_status(status, &format!("{engine_command} run jbotci-render"))
}

#[requires(!command_name.trim().is_empty())]
#[ensures(true)]
fn container_engine_available(command_name: &str) -> bool {
    ProcessCommand::new(command_name)
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|status| status.success())
}

#[requires(true)]
#[ensures(ret.starts_with('/'))]
fn dioxus_runtime_asset_root(base_path: &str) -> String {
    let trimmed = base_path.trim().trim_matches('/');
    if trimmed.is_empty() {
        "/".to_owned()
    } else {
        format!("/{trimmed}")
    }
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn write_release_service_worker(public_dir: &Path) -> Result<()> {
    let precache_paths = release_service_worker_precache_paths(public_dir)?;
    let cache_version = release_service_worker_cache_version(public_dir, &precache_paths)?;
    let contents = render_release_service_worker(&cache_version, &precache_paths)?;
    write_web_asset_text_atomically(
        &public_dir.join(RELEASE_SERVICE_WORKER_FILE_NAME),
        &contents,
        "release service worker",
    )
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|paths| paths.iter().all(|path| !path.starts_with('/'))) || ret.is_err())]
#[ensures(ret.as_ref().is_ok_and(|paths| paths.windows(2).all(|pair| pair[0] <= pair[1])) || ret.is_err())]
fn release_service_worker_precache_paths(public_dir: &Path) -> Result<Vec<String>> {
    if !public_dir.is_dir() {
        bail!(
            "release web public directory `{}` does not exist",
            public_dir.display()
        );
    }
    let mut paths = Vec::new();
    for entry in WalkDir::new(public_dir) {
        let entry = entry
            .with_context(|| format!("walking release web output `{}`", public_dir.display()))?;
        if !entry.file_type().is_file() {
            continue;
        }
        let relative = web_relative_asset_path(public_dir, entry.path())?;
        if release_service_worker_should_precache(&relative) {
            paths.push(relative);
        }
    }
    paths.sort();
    Ok(paths)
}

#[requires(root.is_dir())]
#[requires(path.is_file())]
#[ensures(ret.as_ref().is_ok_and(|path| !path.is_empty() && !path.starts_with('/')) || ret.is_err())]
fn web_relative_asset_path(root: &Path, path: &Path) -> Result<String> {
    let relative = path.strip_prefix(root).with_context(|| {
        format!(
            "making `{}` relative to `{}`",
            path.display(),
            root.display()
        )
    })?;
    let mut parts = Vec::new();
    for component in relative.components() {
        match component {
            std::path::Component::Normal(part) => {
                let text = part.to_str().with_context(|| {
                    format!("release web asset path `{}` is not utf-8", path.display())
                })?;
                parts.push(text);
            }
            _ => bail!(
                "release web asset path `{}` is not normalized",
                path.display()
            ),
        }
    }
    if parts.is_empty() {
        bail!(
            "release web asset path `{}` has no relative components",
            path.display()
        );
    }
    Ok(parts.join("/"))
}

#[requires(!path.is_empty())]
#[ensures(true)]
fn release_service_worker_should_precache(path: &str) -> bool {
    path != RELEASE_SERVICE_WORKER_FILE_NAME
        && !path.ends_with(".br")
        && !path.ends_with(".gz")
        && !path.ends_with(".map")
        && !path.starts_with("assets/embeddings/")
}

#[requires(public_dir.is_dir())]
#[requires(paths.iter().all(|path| !path.is_empty() && !path.starts_with('/')))]
#[ensures(ret.as_ref().is_ok_and(|version| !version.is_empty()) || ret.is_err())]
fn release_service_worker_cache_version(public_dir: &Path, paths: &[String]) -> Result<String> {
    let mut hasher = Sha256::new();
    hasher.update(RELEASE_SERVICE_WORKER_TEMPLATE.as_bytes());
    hasher.update([0]);
    for path in paths {
        let bytes = fs::read(public_dir.join(path))
            .with_context(|| format!("reading release web asset `{path}` for cache version"))?;
        hasher.update(path.as_bytes());
        hasher.update([0]);
        hasher.update(&bytes);
        hasher.update([0xff]);
    }
    let hash = format!("{:x}", hasher.finalize());
    Ok(hash[..16].to_owned())
}

#[requires(!cache_version.is_empty())]
#[requires(precache_paths.iter().all(|path| !path.is_empty() && !path.starts_with('/')))]
#[ensures(ret.as_ref().is_ok_and(|script| script.contains(cache_version)) || ret.is_err())]
fn render_release_service_worker(cache_version: &str, precache_paths: &[String]) -> Result<String> {
    let cache_version_json = serde_json::to_string(cache_version)?;
    let precache_paths_json = serde_json::to_string(precache_paths)?;
    Ok(RELEASE_SERVICE_WORKER_TEMPLATE
        .replace("__CACHE_VERSION_JSON__", &cache_version_json)
        .replace("__PRECACHE_PATHS_JSON__", &precache_paths_json))
}

#[requires(true)]
#[ensures(ret.as_ref().is_err() || ret.as_ref().is_ok_and(|path| path.is_absolute()))]
fn absolute_path(path: &Path) -> Result<PathBuf> {
    if path.is_absolute() {
        Ok(path.to_path_buf())
    } else {
        Ok(std::env::current_dir()
            .context("resolving current directory")?
            .join(path))
    }
}

#[requires(!command.is_empty(), "checked command name must not be empty")]
#[ensures(true)]
fn check_status(status: ExitStatus, command: &str) -> Result<()> {
    if status.success() {
        Ok(())
    } else {
        bail!("`{command}` failed with status {status}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    const LINUX_DESKTOP_TEMPLATE: &str =
        include_str!("../../apps/jbotci-app/bundle/linux/jbotci.desktop.hbs");
    const WINDOWS_WIX_TEMPLATE: &str =
        include_str!("../../apps/jbotci-app/bundle/windows/jbotci.wxs.hbs");
    const DIOXUS_CONFIG: &str = include_str!("../../apps/jbotci-app/Dioxus.toml");

    #[requires(true)]
    #[ensures(true)]
    fn render_test_template(template: &str, replacements: &[(&str, &str)]) -> String {
        let mut rendered = template.to_owned();
        for (key, value) in replacements {
            rendered = rendered.replace(&format!("{{{{{key}}}}}"), value);
        }
        rendered
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn desktop_bundle_normalization_writes_exact_linux_name() {
        let temp = tempdir().expect("temp dir should be created");
        let raw_dir = temp.path().join("raw");
        let output_dir = temp.path().join("out");
        fs::create_dir_all(&raw_dir).expect("raw dir should be created");
        fs::create_dir_all(&output_dir).expect("output dir should be created");
        fs::write(raw_dir.join("jbotci-app_0.1.0_x86_64.AppImage"), b"bundle")
            .expect("raw appimage should be written");

        let published =
            publish_desktop_bundle_artifact(DesktopBundleTarget::Linux, &raw_dir, &output_dir)
                .expect("artifact should publish");

        assert_eq!(
            published.file_name(),
            Some(OsStr::new(DESKTOP_BUNDLE_LINUX_ARTIFACT))
        );
        assert_eq!(
            fs::read(published).expect("published appimage should be readable"),
            b"bundle"
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn desktop_bundle_normalization_copies_macos_app_directory() {
        let temp = tempdir().expect("temp dir should be created");
        let raw_dir = temp.path().join("raw");
        let output_dir = temp.path().join("out");
        let app_contents = raw_dir.join("JbotciApp.app").join("Contents");
        fs::create_dir_all(&app_contents).expect("raw app contents should be created");
        fs::create_dir_all(&output_dir).expect("output dir should be created");
        fs::write(app_contents.join("Info.plist"), b"plist").expect("plist should be written");

        let published =
            publish_desktop_bundle_artifact(DesktopBundleTarget::Macos, &raw_dir, &output_dir)
                .expect("artifact should publish");

        assert_eq!(
            published.file_name(),
            Some(OsStr::new(DESKTOP_BUNDLE_MACOS_ARTIFACT))
        );
        assert_eq!(
            fs::read(published.join("Contents").join("Info.plist"))
                .expect("published plist should be readable"),
            b"plist"
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn desktop_bundle_raw_artifact_errors_when_missing() {
        let temp = tempdir().expect("temp dir should be created");

        let error = find_desktop_bundle_raw_artifact(DesktopBundleTarget::Windows, temp.path())
            .expect_err("missing artifact should fail");

        assert!(error.to_string().contains("could not find"));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn desktop_bundle_raw_artifact_errors_when_ambiguous() {
        let temp = tempdir().expect("temp dir should be created");
        fs::write(temp.path().join("first.msi"), b"one").expect("first msi should be written");
        fs::write(temp.path().join("second.msi"), b"two").expect("second msi should be written");

        let error = find_desktop_bundle_raw_artifact(DesktopBundleTarget::Windows, temp.path())
            .expect_err("multiple artifacts should fail");

        assert!(error.to_string().contains("expected exactly one"));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn linux_desktop_template_uses_jbotci_name_and_launcher_exec() {
        let rendered = render_test_template(
            LINUX_DESKTOP_TEMPLATE,
            &[
                ("categories", "Reference"),
                ("exec", "jbotci-app"),
                ("icon", "jbotci"),
            ],
        );

        assert!(rendered.contains("Name=jbotci"));
        assert!(rendered.contains("Exec=jbotci-app"));
        assert!(!rendered.contains("Name={{name}}"));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn windows_wix_template_keeps_jbotci_app_executable_variable() {
        let rendered = render_test_template(
            WINDOWS_WIX_TEMPLATE,
            &[("main_binary_name", "jbotci-app.exe")],
        );

        assert!(rendered.contains(r#"Name="jbotci""#));
        assert!(rendered.contains(r#"Name="jbotci-app.exe""#));
        assert!(rendered.contains(r#"Target="[INSTALLDIR]jbotci-app.exe""#));
        assert!(!rendered.contains(r#"Name="{{product_name}}""#));
        assert!(!rendered.contains(r#"Title="{{product_name}}""#));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn dioxus_config_pins_public_desktop_bundle_names() {
        assert!(DIOXUS_CONFIG.contains("bundle_name = \"jbotci\""));
        assert!(DIOXUS_CONFIG.contains("upgrade_code = \"5fc2c34f-0231-53b4-af97-bd710d8abad2\""));
        assert!(
            DIOXUS_CONFIG
                .contains("desktop_template = \"apps/jbotci-app/bundle/linux/jbotci.desktop.hbs\"")
        );
        assert!(DIOXUS_CONFIG.contains("template = \"bundle/windows/jbotci.wxs.hbs\""));
    }
}

const RELEASE_SERVICE_WORKER_TEMPLATE: &str = r#"const CACHE_VERSION = __CACHE_VERSION_JSON__;
const STATIC_CACHE_NAME = `jbotci-static-${CACHE_VERSION}`;
const RUNTIME_CACHE_NAME = `jbotci-runtime-${CACHE_VERSION}`;
const CURRENT_CACHE_NAMES = new Set([STATIC_CACHE_NAME, RUNTIME_CACHE_NAME]);
const PRECACHE_PATHS = __PRECACHE_PATHS_JSON__;

const SCOPE_URL = new URL(self.registration.scope);
if (!SCOPE_URL.pathname.endsWith("/")) {
  SCOPE_URL.pathname = `${SCOPE_URL.pathname}/`;
}
const APP_SHELL_URL = new URL("index.html", SCOPE_URL).href;
const PRECACHE_URLS = new Set(
  PRECACHE_PATHS.map((path) => new URL(path, SCOPE_URL).href),
);

self.addEventListener("install", (event) => {
  event.waitUntil((async () => {
    const cache = await caches.open(STATIC_CACHE_NAME);
    await cache.addAll(
      PRECACHE_PATHS.map((path) => new Request(new URL(path, SCOPE_URL), {
        cache: "default",
      })),
    );
    await self.skipWaiting();
  })());
});

self.addEventListener("activate", (event) => {
  event.waitUntil((async () => {
    const cacheNames = await caches.keys();
    await Promise.all(cacheNames.map((name) => {
      if (name.startsWith("jbotci-") && !CURRENT_CACHE_NAMES.has(name)) {
        return caches.delete(name);
      }
      return Promise.resolve(false);
    }));
    await self.clients.claim();
  })());
});

self.addEventListener("fetch", (event) => {
  const request = event.request;
  if (request.method !== "GET") {
    return;
  }

  const url = new URL(request.url);
  if (url.origin !== self.location.origin) {
    return;
  }

  const relativePath = relativeScopedPath(url);
  if (relativePath === null) {
    return;
  }

  if (isApiRequest(relativePath)) {
    event.respondWith(networkOnlyJson(request));
    return;
  }

  if (isEmbeddingAssetRequest(relativePath)) {
    return;
  }

  if (request.mode === "navigate") {
    event.respondWith(networkFirst(request, RUNTIME_CACHE_NAME, APP_SHELL_URL));
    return;
  }

  if (PRECACHE_URLS.has(url.href)) {
    event.respondWith(networkFirst(request, STATIC_CACHE_NAME, null));
    return;
  }

  if (isStaticOrCoreRequest(relativePath)) {
    event.respondWith(networkFirst(request, RUNTIME_CACHE_NAME, null));
  }
});

function relativeScopedPath(url) {
  if (!url.pathname.startsWith(SCOPE_URL.pathname)) {
    return null;
  }
  return url.pathname.slice(SCOPE_URL.pathname.length);
}

function isApiRequest(relativePath) {
  return relativePath === "api" || relativePath.startsWith("api/");
}

function isEmbeddingAssetRequest(relativePath) {
  return relativePath.startsWith("assets/embeddings/");
}

function isStaticOrCoreRequest(relativePath) {
  return relativePath === ""
    || relativePath === "index.html"
    || relativePath === "manifest.webmanifest"
    || relativePath === "service-worker.js"
    || relativePath.startsWith("assets/");
}

async function networkFirst(request, cacheName, fallbackUrl) {
  const cache = await caches.open(cacheName);
  try {
    const response = await fetch(request);
    if (response.ok && response.type !== "opaque") {
      await cache.put(request, response.clone());
    }
    return response;
  } catch (error) {
    const cached = await caches.match(request);
    if (cached) {
      return cached;
    }
    if (fallbackUrl !== null) {
      const fallback = await caches.match(fallbackUrl);
      if (fallback) {
        return fallback;
      }
    }
    return offlineTextResponse();
  }
}

async function networkOnlyJson(request) {
  try {
    return await fetch(request);
  } catch (error) {
    return new Response(JSON.stringify({
      error: "offline",
      message: "jbotci is offline and this API request is not cached.",
    }), {
      status: 503,
      headers: {
        "Content-Type": "application/json; charset=utf-8",
      },
    });
  }
}

function offlineTextResponse() {
  return new Response("jbotci is offline and this resource is not cached.", {
    status: 503,
    headers: {
      "Content-Type": "text/plain; charset=utf-8",
    },
  });
}
"#;

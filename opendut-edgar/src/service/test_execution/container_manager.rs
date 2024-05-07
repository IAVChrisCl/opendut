use std::{env, fmt, io::Cursor, path::{Path, PathBuf}};

//use anyhow::{bail, Error};
//use anyhow::{bail, Context, Result};
use tokio::{fs, process::Command};
use tracing::{error, warn};
use uuid::Uuid;
use zip::ZipWriter;
use zip_extensions::write::ZipWriterExtensions;

use super::webdav_client::WebdavClient;
//use tracing::error;

pub struct ContainerId {
    value: String
}

impl fmt::Display for ContainerId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.value)
    }
}

enum ContainerState {
    Created,
    Running,
    Restarting,
    Exited,
    Paused,
    Dead,
}

pub struct DummyConfig {
    engine: String,
    image_name: String,
    
}

pub struct ContainerManager{
    config: DummyConfig,
    container_id: Option<ContainerId>,
    results_dir: PathBuf,
    webdav_client: WebdavClient,
}

const MONITOR_INTERVAL_MS: u64 = 1000;
const RESULTS_READY_FILE: &str = ".results_ready";
const CONTAINER_RESULTS_DIRECTORY: &str = "/results";

impl ContainerManager {

    pub fn new(config: DummyConfig) -> Self {
        Self { 
            config,
            container_id: None,
            results_dir: env::temp_dir().join(format!("opendut-edgar-results_{}", Uuid::new_v4())),
            webdav_client: WebdavClient::new(),
        }
    }

    pub async fn start(&mut self) {
        match self.run().await {
            Ok(_) => (),
            Err(cause) => error!("{}", cause.to_string()),
        }
    }

    async fn run(&mut self) -> Result<(), Error> {
        self.create_results_dir().await?;
        self.start_container().await?;

        loop {
            match self.get_container_state().await? {
                ContainerState::Running => (),
                ContainerState::Created | ContainerState::Restarting | ContainerState::Paused | ContainerState::Dead => {
                    warn!("Unexpected container state.")
                },
                ContainerState::Exited => {
                    self.upload_results().await?;
                    break
                },
            }
            if self.are_results_ready().await? {
                self.upload_results().await?;
            }

            tokio::time::sleep(std::time::Duration::from_millis(MONITOR_INTERVAL_MS)).await;
        }

        self.cleanup_results_dir().await?;

        Ok(())
    }

    async fn get_container_state(&self) -> Result<ContainerState, Error> {
        match &self.container_id {
            Some(id) => {
                let output = Command::new("docker")
                    .args(["inspect", "-f", "'{{.State.Status}}'", &id.to_string()])
                    .output()
                    .await
                    .map_err(|cause| Error::CommandLineProgramExecution { command: "docker inspect".to_string(), cause })?;

                match String::from_utf8_lossy(&output.stdout).into_owned().trim() {
                    "created" => Ok(ContainerState::Created),
                    "running" => Ok(ContainerState::Running),
                    "restarting" => Ok(ContainerState::Restarting),
                    "exited" => Ok(ContainerState::Exited),
                    "paused" => Ok(ContainerState::Paused),
                    "dead" => Ok(ContainerState::Dead),
                    unknown_state => Err(Error::Other { message: format!("Unknown container state returned by docker inspect: '{}'", unknown_state) } ),
                }
            },
            None => Err(Error::Other { message: "get_container_state() called without container_id present".to_string()}),
        }
        
    }

    async fn start_container(&self) -> Result<(), Error>{
        let engine = "docker";
        let image = "someimage";
        let name = "somename";

        let mut cmd = Command::new(engine);
        cmd.arg("run");
        cmd.arg("--restart=unless-stopped");

        cmd.arg("--net=host");

        let mut container_name = String::new();
        for ctr in 1..i32::MAX {
            container_name = format!("{name}-{ctr}");
            if ! self.check_container_name_exists(&container_name).await? {
                break;
            }
        }
        cmd.args(["--name", container_name.as_str()]);
        
        cmd.args([format!("--mount=type=bind,source={}),target={}", self.results_dir.to_string_lossy(), CONTAINER_RESULTS_DIRECTORY)]);
        
        // TODO: Add environment variables and arguments

        match cmd.spawn() {
            Ok(_) => { }
            Err(_) => { error!("Failed to start container.") }
        };

        Ok(())
    }

    async fn check_container_name_exists(&self, name: &str) -> Result<bool, Error>{
        let status = Command::new("docker")
            .args(["container", "inspect", name])
            .status()
            .await
            .map_err(|cause| Error::CommandLineProgramExecution { command: "docker inspect".to_string(), cause })?;

        Ok(status.success())
    }

    async fn stop_container(&self) -> Result<(), Error>{
        match &self.container_id {
            Some(id) => {
                let output = Command::new("docker")
                    .args(["stop", &id.to_string()])
                    .output()
                    .await
                    .map_err(|cause| Error::CommandLineProgramExecution { command: "docker stop".to_string(), cause })?;

                match output.status.success() {
                    true => Ok(()),
                    false => Err(Error::Other { message: format!("Stopping container failed: {}", String::from_utf8_lossy(&output.stderr)).to_string()})
                }

            },
            None => Err(Error::Other { message: "stop_container() called without container_id present".to_string()}),
        }
    }

    async fn upload_results(&self) -> Result<(), Error>{

        let mut indicator_file = self.results_dir.clone();
        indicator_file.push(RESULTS_READY_FILE);
        fs::remove_file(&indicator_file)
            .await
            .map_err(|cause| Error::Other { message: format!("Failed to remove result indicator file '{}': {}", indicator_file.to_string_lossy(), cause) })?;

        
        let mut data = Vec::new();
        let buffer = Cursor::new(&mut data);
        let mut zip = ZipWriter::new(buffer);

        zip.create_from_directory(&self.results_dir)
            .map_err(|cause| Error::ResultZipping { path: self.results_dir.clone(), cause })?;

        let results_url = "asdsad";

        let zipped_data = zip.finish()
            .map_err(|cause| Error::ResultZipping { path: self.results_dir.clone(), cause })?
            .into_inner().to_owned();
        
        self.webdav_client.put(zipped_data, results_url)
            .await
            .map_err(|cause| Error::ResultUploading { path: String::from(results_url), cause })?;

        Ok(())
    }

    async fn create_results_dir(&mut self) -> Result<(), Error>{
        let dirname = format!("opendut-edgar-results_{}", Uuid::new_v4());
        let results_dir = env::temp_dir().join(dirname);
        
        fs::create_dir(&results_dir)
            .await
            .map_err(|cause| Error::Other { message: format!("Failed to create results directory '{}': {}", results_dir.to_string_lossy(), cause) })?;

        Ok(())
    }

    async fn cleanup_results_dir(&self) -> Result<(), Error> {

        fs::remove_dir_all(&self.results_dir)
            .await
            .map_err(|cause| Error::Other { message: format!("Failed to remove results directory '{}': {}", self.results_dir.to_string_lossy(), cause) })?;
        Ok(())

        
    }

    async fn are_results_ready(&self) -> Result<bool, Error> {
        let mut indicator_file = self.results_dir.clone();
        indicator_file.push(RESULTS_READY_FILE);
        Ok(indicator_file.is_file())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Failure while invoking command line program '{command}': {cause}")]
    CommandLineProgramExecution { command: String, cause: std::io::Error },
    #[error("Failure while creating a ZIP archive of the test results at '{path}' : {cause}")]
    ResultZipping { path: PathBuf, cause: zip::result::ZipError },
    #[error("Failure while uploading test results to '{path}': {cause}")]
    ResultUploading { path: String, cause: reqwest::Error },
    #[error("{message}")]
    Other { message: String },
}
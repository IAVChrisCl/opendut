use uuid::Uuid;

use opendut_carl_api::carl::CarlClient;
use opendut_types::peer::{PeerId};
use opendut_types::peer::executor::{ContainerCommand, ContainerCommandArgument, ContainerDevice, ContainerEnvironmentVariable, ContainerImage, ContainerName, ContainerPortSpec, ContainerVolume, Engine, ExecutorDescriptor};

use crate::commands::peer;
use crate::{CreateOutputFormat, DescribeOutputFormat, EngineVariants};

/// Create a container executor
#[derive(clap::Parser)]
pub struct CreateContainerExecutorCli {
    #[clap(subcommand)]
    pub option: Option,
}

#[derive(clap::Parser)]
pub enum Option {
    JsonConfigString(JsonConfigString),
    CommandLineArguments(CommandLineArguments),
}

#[derive(clap::Parser)]
pub struct JsonConfigString {
    ///Input file
    #[clap(short, long)]
    test_executor_config: String,
}

#[derive(clap::Parser)]
pub struct CommandLineArguments {
    ///ID of the peer to add the container executor to
    #[arg(long)]
    peer_id: Uuid,
    ///Engine
    #[arg(short, long)]
    engine: EngineVariants,
    ///Container name
    #[arg(short, long)]
    name: Option<ContainerName>,
    ///Container image
    #[arg(short, long)]
    image: ContainerImage,
    ///Container volumes
    #[arg(short, long, num_args = 1..)]
    volumes: Option<Vec<ContainerVolume>>,
    ///Container devices
    #[arg(long, num_args = 1..)]
    devices: Option<Vec<ContainerDevice>>,
    ///Container envs
    #[arg(long, num_args = 1..)]
    envs: Option<Vec<String>>,
    ///Container ports
    #[arg(short, long, num_args = 1..)]
    ports: Option<Vec<ContainerPortSpec>>,
    ///Container command
    #[arg(short, long)]
    command: Option<ContainerCommand>,
    ///Container arguments
    #[arg(short, long, num_args = 1..)]
    args: Option<Vec<ContainerCommandArgument>>,
}

impl CreateContainerExecutorCli {
    #[allow(clippy::too_many_arguments)]
    pub async fn execute(self, carl: &mut CarlClient, output: CreateOutputFormat) -> crate::Result<()> {
        match self.option {
            Some(Option::JsonConfigString(json_string)) => {
                let executor_descriptor: ExecutorDescriptor = serde_json::from_str(json_string)?;
                let peer_id = executor_descriptor.peer_id;
                let mut peer_descriptor = carl.peers.get_peer_descriptor(peer_id).await
                    .map_err(|_| format!("Failed to get peer with ID <{}>.", peer_id))?;   
            }
            Some(Option::CommandLineArguments(args)) => {

                let peer_id = PeerId::from(args.peer_id);

                let engine = match args.engine {
                    EngineVariants::Docker => { Engine::Docker }
                    EngineVariants::Podman => { Engine::Podman }
                };

                let volumes = args.volumes.unwrap_or_default();
                let devices = args.devices.unwrap_or_default();
                let ports = args.ports.unwrap_or_default();
                let args = args.args.unwrap_or_default();

                let mut environment_variables = vec![];

                for env in args.envs.unwrap_or_default() {
                    if let Some((name, value)) = env.split_once('=') {
                        let env = ContainerEnvironmentVariable::new(name, value)
                            .map_err(|cause| cause.to_string())?;
                        environment_variables.push(env)
                    }
                };

                let executor_descriptor = ExecutorDescriptor::Container {
                    engine,
                    name: args.name.unwrap_or_default(),
                    image: args.image,
                    volumes,
                    devices,
                    envs: environment_variables,
                    ports,
                    command: args.command.unwrap_or_default(),
                    args,
                };

                
            }
        }
        let mut peer_descriptor = carl.peers.get_peer_descriptor(peer_id).await
                    .map_err(|_| format!("Failed to get peer with ID <{}>.", peer_id))?;

        peer_descriptor.executors.executors.push(executor_descriptor);

        carl.peers.store_peer_descriptor(Clone::clone(&peer_descriptor)).await
            .map_err(|error| format!("Failed to update peer <{}>.\n  {}", peer_id, error))?;
        let output_format = DescribeOutputFormat::from(output);
        crate::commands::peer::describe::render_peer_descriptor(peer_descriptor, output_format);

        Ok(())
    }
}

use crate::proto::{ConversionError, ConversionErrorBuilder};

include!(concat!(env!("OUT_DIR"), "/opendut.types.peer.configuration.rs"));

impl From<crate::peer::configuration::PeerConfiguration> for PeerConfiguration {
    fn from(value: crate::peer::configuration::PeerConfiguration) -> Self {
        Self {
            executors: Some(value.executors.into()),
            cluster_assignment: value.cluster_assignment.map(|assignment| assignment.into()),
        }
    }
}
impl TryFrom<PeerConfiguration> for crate::peer::configuration::PeerConfiguration {
    type Error = ConversionError;

    fn try_from(value: PeerConfiguration) -> Result<Self, Self::Error> {
        type ErrorBuilder = ConversionErrorBuilder<PeerConfiguration, crate::peer::configuration::PeerConfiguration>;

        let executors = value.executors
            .ok_or(ErrorBuilder::field_not_set("executors"))?
            .try_into()?;

        let cluster_assignment = value.cluster_assignment
            .map(TryInto::try_into)
            .transpose()?;

        Ok(crate::peer::configuration::PeerConfiguration {
            executors,
            cluster_assignment
        })
    }
}


impl From<crate::peer::configuration::PeerConfiguration2> for PeerConfiguration2 {
    fn from(value: crate::peer::configuration::PeerConfiguration2) -> Self {
        Self {
            executors: value.executors.into_iter().map(PeerConfigurationParameterExecutor::from).collect(),
        }
    }
}
impl TryFrom<PeerConfiguration2> for crate::peer::configuration::PeerConfiguration2 {
    type Error = ConversionError;

    fn try_from(value: PeerConfiguration2) -> Result<Self, Self::Error> {
        Ok(crate::peer::configuration::PeerConfiguration2 {
            executors: value.executors.into_iter().map(TryInto::try_into).collect::<Result<_, _>>()?,
        })
    }
}

impl From<crate::peer::configuration::Parameter<crate::peer::executor::ExecutorDescriptor>> for PeerConfigurationParameterExecutor {
    fn from(value: crate::peer::configuration::Parameter<crate::peer::executor::ExecutorDescriptor>) -> Self {

        let executor: crate::proto::peer::executor::ExecutorDescriptor = value.value.clone().into();
        let parameter = PeerConfigurationParameter::from(value);

        Self {
            parameter: Some(parameter),
            value: Some(executor),
        }
    }
}
impl TryFrom<PeerConfigurationParameterExecutor> for crate::peer::configuration::Parameter<crate::peer::executor::ExecutorDescriptor> {
    type Error = ConversionError;

    fn try_from(value: PeerConfigurationParameterExecutor) -> Result<Self, Self::Error> {
        type ErrorBuilder = ConversionErrorBuilder<PeerConfigurationParameterExecutor, crate::peer::configuration::Parameter<crate::peer::executor::ExecutorDescriptor>>;

        let parameter = value.parameter
            .ok_or(ErrorBuilder::field_not_set("parameter"))?;

        let executor: crate::peer::executor::ExecutorDescriptor = value.value
            .ok_or(ErrorBuilder::field_not_set("executor"))?
            .try_into()?;

        Ok(Self {
            id: parameter.id.ok_or(ErrorBuilder::field_not_set("id"))?.try_into()?,
            dependencies: parameter.dependencies.into_iter().map(TryInto::try_into).collect::<Result<_, _>>()?,
            target: parameter.target.ok_or(ErrorBuilder::field_not_set("target"))?.into(),
            value: executor,
        })
    }
}

impl<V: crate::peer::configuration::ParameterValue> From<crate::peer::configuration::Parameter<V>> for PeerConfigurationParameter {
    fn from(value: crate::peer::configuration::Parameter<V>) -> Self {
        Self {
            id: Some(value.id.into()),
            dependencies: value.dependencies.into_iter().map(Into::into).collect(),
            target: Some(value.target.into()),
        }
    }
}

impl From<crate::peer::configuration::ParameterId> for PeerConfigurationParameterId {
    fn from(value: crate::peer::configuration::ParameterId) -> Self {
        Self {
            uuid: Some(value.0.into())
        }
    }
}
impl TryFrom<PeerConfigurationParameterId> for crate::peer::configuration::ParameterId {
    type Error = ConversionError;

    fn try_from(value: PeerConfigurationParameterId) -> Result<Self, Self::Error> {
        type ErrorBuilder = ConversionErrorBuilder<PeerConfigurationParameterId, crate::peer::configuration::ParameterId>;

        value.uuid
            .ok_or(ErrorBuilder::field_not_set("uuid"))
            .map(|uuid| Self(uuid.into()))
    }
}

impl From<crate::peer::configuration::ParameterTarget> for peer_configuration_parameter::Target {
    fn from(value: crate::peer::configuration::ParameterTarget) -> Self {
        match value {
            crate::peer::configuration::ParameterTarget::Present => peer_configuration_parameter::Target::Present(PeerConfigurationParameterTargetPresent {}),
            crate::peer::configuration::ParameterTarget::Absent => peer_configuration_parameter::Target::Absent(PeerConfigurationParameterTargetAbsent {}),
        }
    }
}
impl From<peer_configuration_parameter::Target> for crate::peer::configuration::ParameterTarget {
    fn from(value: peer_configuration_parameter::Target) -> Self {
        match value {
            peer_configuration_parameter::Target::Present(_) => crate::peer::configuration::ParameterTarget::Present,
            peer_configuration_parameter::Target::Absent(_) => crate::peer::configuration::ParameterTarget::Absent,
        }
    }
}

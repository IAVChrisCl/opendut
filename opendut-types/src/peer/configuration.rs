use std::any::Any;
use std::hash::{DefaultHasher, Hash, Hasher};

use uuid::Uuid;

use crate::cluster::ClusterAssignment;
use crate::OPENDUT_UUID_NAMESPACE;
use crate::peer::executor::{ExecutorDescriptor, ExecutorDescriptors};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PeerConfiguration {
    pub executors: ExecutorDescriptors,
    pub cluster_assignment: Option<ClusterAssignment>,
}


#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PeerConfiguration2 {
    pub executors: Vec<Parameter<ExecutorDescriptor>>,
}
impl PeerConfiguration2 {
    pub fn insert_executor(&mut self, value: ExecutorDescriptor, target: ParameterTarget) { //TODO more generic solution
        let id = calculate_id(&value);

        let parameter = Parameter {
            id,
            dependencies: vec![], //TODO
            target,
            value,
        };

        self.executors.push(parameter);
    }
}

fn calculate_id(value: &impl ParameterValue) -> ParameterId {
    let mut hasher = DefaultHasher::new();
    value.hash(&mut hasher);
    let id = hasher.finish();

    let id = Uuid::new_v5(&OPENDUT_UUID_NAMESPACE, &id.to_le_bytes());
    ParameterId(id)
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Parameter<V: ParameterValue> {
    pub id: ParameterId,
    pub dependencies: Vec<ParameterId>,
    pub target: ParameterTarget,
    pub value: V,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ParameterId(pub Uuid);
impl ParameterId {
    pub fn random() -> Self {
        Self(Uuid::new_v4())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ParameterTarget {
    Present,
    Absent,
}

pub trait ParameterValue: Any + Hash {}
impl ParameterValue for ExecutorDescriptor {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn insert_value_in_peer_configuration2() {
        let mut peer_configuration = PeerConfiguration2::default();

        let value = ExecutorDescriptor::Executable;
        let target = ParameterTarget::Present;
        peer_configuration.insert_executor(value.clone(), target);

        assert_eq!(peer_configuration.executors.len(), 1);

        let executor_target = peer_configuration.executors.first().unwrap();
        assert_eq!(executor_target.value, value);
        assert_eq!(executor_target.target, target);
    }
}

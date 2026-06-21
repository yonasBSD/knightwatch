use serde_json::{Value, json};

use crate::{
    docker_tracker::DockerTrackerEvent, process_tracker::ProcessTrackerEvent,
    system_resources::SystemResourcesEvent, systemd::SystemdEvent,
};


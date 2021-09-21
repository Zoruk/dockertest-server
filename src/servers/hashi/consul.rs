use crate::common::rand_string;
use crate::{Config, ContainerConfig, Server};
use derive_builder::Builder;
use dockertest::{waitfor, PullPolicy, Source};
use std::collections::HashMap;

const IMAGE: &str = "consul";
const PORT: u32 = 8500;
const LOG_MSG: &str = "Synced node info";
const SOURCE: Source = Source::DockerHub(PullPolicy::IfNotPresent);

/// Configuration for creating a Hashicorp Consul server.
///
/// By default the Consul server listens on port 8500 for HTTP requests. This
/// is exposed on the container by default, but the exposed port can be
/// controlled by setting the `port` field.
///
/// See the [Dockerhub](https://hub.docker.com/_/consul) page for more
/// information on the arguments and environment variables that can be used to
/// configure the server.
#[derive(Clone, Default, Builder)]
#[builder(default)]
pub struct ConsulServerConfig {
    #[builder(default = "Vec::new()")]
    pub args: Vec<String>,
    #[builder(default = "HashMap::new()")]
    pub env: HashMap<String, String>,
    #[builder(default = "crate::server::new_handle(IMAGE)")]
    pub handle: String,
    #[builder(default = "9500")]
    pub port: u32,
    #[builder(default = "15")]
    pub timeout: u16,
    #[builder(default = "rand_string(16)")]
    pub token: String,
    #[builder(default = "String::from(\"latest\")")]
    pub version: String,
}

impl ConsulServerConfig {
    pub fn builder() -> ConsulServerConfigBuilder {
        ConsulServerConfigBuilder::default()
    }
}

impl Config for ConsulServerConfig {
    fn into_composition(self) -> dockertest::Composition {
        let ports = vec![(PORT, self.port)];

        let timeout = self.timeout;
        let wait = Box::new(waitfor::MessageWait {
            message: LOG_MSG.into(),
            source: waitfor::MessageSource::Stdout,
            timeout,
        });

        ContainerConfig {
            args: self.args,
            env: self.env,
            handle: self.handle,
            name: IMAGE.into(),
            source: SOURCE,
            version: self.version,
            ports: Some(ports),
            wait: Some(wait),
        }
        .into()
    }

    fn handle(&self) -> &str {
        self.handle.as_str()
    }
}

/// A running instance of a Consul server.
///
/// The server URL which is accessible from the local host can be found in
/// `local_address`. Other running containers which need access to this server
/// should use the `address` field instead.
pub struct ConsulServer {
    pub address: String,
    pub external_port: u32,
    pub internal_port: u32,
    pub local_address: String,
}

impl Server for ConsulServer {
    type Config = ConsulServerConfig;

    fn new(config: &Self::Config, container: &dockertest::RunningContainer) -> Self {
        ConsulServer {
            address: format!("http://{}:{}", container.ip(), PORT),
            external_port: config.port,
            internal_port: PORT,
            local_address: format!("http://localhost:{}", config.port),
        }
    }
}

#[cfg(test)]
mod tests {

    use super::{ConsulServer, ConsulServerConfig};
    use crate::Test;

    #[test]
    fn test_consul() {
        let config = ConsulServerConfig::builder()
            .port(9500)
            .version("1.9.9".into())
            .build()
            .unwrap();
        let mut test = Test::new();
        test.register(config);

        test.run(|instance| async move {
            let server: ConsulServer = instance.server();

            let client = reqwest::Client::new();
            let resp = client.get(server.local_address).send().await;
            assert!(resp.is_ok());
            assert_eq!(resp.unwrap().status(), 200);
        });
    }
}

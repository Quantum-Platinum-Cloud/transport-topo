use crate::api_client::ApiClient;
use crate::sparql_client::SparqlClient;
use failure::Error;
use log::{error, info, warn};
use serde::Deserialize;
use std::io::Read;

#[derive(Deserialize, Debug, Clone, Default)]
pub struct Config {
    pub api_endpoint: String,
    pub sparql_endpoint: String,
    pub properties: Properties,
    pub items: Items,
}

#[derive(Deserialize, Debug, Clone, Default)]
pub struct Properties {
    pub produced_by: String,
    pub instance_of: String,
    pub physical_mode: String,
    pub gtfs_short_name: String,
    pub gtfs_long_name: String,
    pub gtfs_id: String,
}

#[derive(Deserialize, Debug, Clone, Default)]
pub struct Items {
    pub line: u64,
    pub producer: u64,
    pub bus: u64,
}

pub struct Client {
    pub api: ApiClient,
    pub sparql: SparqlClient,
}

impl Config {
    pub fn physical_mode(&self, route: &gtfs_structures::Route) -> u64 {
        use gtfs_structures::RouteType::*;
        match route.route_type {
            Bus => self.items.bus,
            _ => 6,
        }
    }
}

impl Client {
    pub fn from_config_file<P: AsRef<std::path::Path>>(config_file: P) -> Result<Self, Error> {
        let mut f = std::fs::File::open(config_file)?;
        let mut content = String::new();
        f.read_to_string(&mut content)?;
        let config = toml::from_str::<Config>(&content)?;
        Self::new(config)
    }

    pub fn new(config: Config) -> Result<Self, Error> {
        Ok(Self {
            api: ApiClient::new(config.clone())?,
            sparql: SparqlClient::new(config),
        })
    }

    pub fn import_lines(
        &mut self,
        gtfs_filename: &str,
        producer_id: &str,
        producer_name: &str,
    ) -> Result<(), failure::Error> {
        let gtfs = gtfs_structures::RawGtfs::from_zip(gtfs_filename)?;
        let routes = gtfs.routes?;

        for route in routes {
            let r = self.sparql.find_line(producer_id, &route.id)?;
            match r.len() {
                0 => {
                    info!(
                        "Line “{}” ({}) does not exist, inserting",
                        route.long_name, route.short_name
                    );
                    match self.api.insert_route(producer_id, producer_name, &route) {
                        Ok(res) => info!("Ok, new item id: {}", res),
                        Err(e) => error!("Insertion failed: {}", e),
                    }
                }
                1 => {
                    info!(
                        "Line “{}” ({}) already exists with id {}, skipping",
                        route.long_name, route.short_name, r[0]["line"]
                    );
                }
                _ => warn!(
                    "Line “{}” ({}) exists many times. Something is not right",
                    route.long_name, route.short_name
                ),
            }
        }
        Ok(())
    }
}

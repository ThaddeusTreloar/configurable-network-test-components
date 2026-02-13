use std::{
    collections::{HashMap, HashSet},
    net::{SocketAddr, ToSocketAddrs},
    sync::{Arc, atomic::AtomicBool},
};

use bb8::Pool;
use http_body_util::Empty;
use hyper::body::{Body, Bytes};
use tokio::sync::RwLock;

use crate::{
    connection_manager::{ConnectionManager, ConnectionManagerError},
    connection_pool,
    target::TargetGroup,
};

#[derive(Debug, thiserror::Error)]
pub enum ConnectionPoolCreationError {
    #[error("Failed to create connection pool for target group: {0}, due to error: {1}")]
    PoolCreation(String, ConnectionManagerError),
    #[error("Failed to get socket address for target group: {0}, due to error: {1}")]
    SocketAddressCreation(String, std::io::Error),
    #[error("Failed to create health check pools: {0}")]
    CreateHealthCheckPool(TargetConnectionPoolCloneError),
}

pub struct TargetGroupsConnectionPools<T>
where
    T: Send + Sync + Body + 'static,
    T::Data: Send,
    T::Error: Into<Box<dyn serde::ser::StdError + Send + Sync>>,
{
    pub groups_connection_pools: HashMap<String, Arc<RwLock<Vec<TargetConnectionPool<T>>>>>,
}

impl<T> TargetGroupsConnectionPools<T>
where
    T: Send + Sync + Body + 'static,
    T::Data: Send,
    T::Error: Into<Box<dyn serde::ser::StdError + Send + Sync>>,
{
    pub async fn create_health_check_pools(
        &self,
    ) -> Result<HashMap<String, Vec<TargetConnectionPool<Empty<Bytes>>>>, ConnectionPoolCreationError>
    {
        let mut groups_health_check_connection_pools = HashMap::new();

        for (group_name, connection_pool) in self.groups_connection_pools.iter() {
            let mut group_health_check_connection_pools = Vec::new();

            let connection_pool_guard = connection_pool.read().await;

            for pool in connection_pool_guard.iter() {
                let health_check_pool = pool
                    .create_health_check_pool()
                    .await
                    .map_err(ConnectionPoolCreationError::CreateHealthCheckPool)?;

                group_health_check_connection_pools.push(health_check_pool);
            }
            groups_health_check_connection_pools
                .insert(group_name.clone(), group_health_check_connection_pools);
        }

        Ok(groups_health_check_connection_pools)
    }

    // pub fn unwrap(self) -> HashMap<String, Vec<TargetConnectionPool<T>>> {
    //     self.groups_connection_pools
    //         .into_iter()
    //         .map(|(k, v)| {
    //             (
    //                 k,
    //                 Arc::into_inner(v)
    //                     .map(RwLock::into_inner)
    //                     .expect("Failed to unwrap locks"),
    //             )
    //         })
    //         .collect()
    // }

    pub fn get_pool_for_group(
        &self,
        target_group: &str,
    ) -> Option<Arc<RwLock<Vec<TargetConnectionPool<T>>>>> {
        self.groups_connection_pools
            .get(target_group)
            .map(Clone::clone)
    }

    pub async fn try_from_target_groups(
        targets: &HashMap<String, TargetGroup>,
        pool_size: u32,
    ) -> Result<Self, ConnectionPoolCreationError> {
        let mut connection_pools = HashMap::new();

        for (group_name, target_group) in targets.iter() {
            let socked_addrs = target_group
                .targets
                .iter()
                .map(|t| {
                    (t.hostname.as_ref(), t.port)
                        .to_socket_addrs()
                        .map(|s| s.collect::<HashSet<SocketAddr>>())
                        .map(|s| (s, t.uri.clone()))
                })
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| {
                    ConnectionPoolCreationError::SocketAddressCreation(group_name.clone(), e)
                })?
                .into_iter()
                .flat_map(|(s, u)| s.into_iter().map(move |s| (s, u.clone())));

            let mut connections = Vec::new();

            for (socket, uri) in socked_addrs {
                connections.push(TargetConnectionPool {
                    connection_pool: Pool::builder()
                        .max_size(pool_size)
                        .build(ConnectionManager::new(socket))
                        .await
                        .map_err(|e| {
                            ConnectionPoolCreationError::PoolCreation(group_name.clone(), e)
                        })?,
                    _socket_addr: socket,
                    uri,
                });
            }

            connection_pools.insert(group_name.to_owned(), Arc::new(RwLock::new(connections)));
        }

        Ok(Self {
            groups_connection_pools: connection_pools,
        })
    }
}

pub struct TargetConnectionPool<T>
where
    T: Send + Sync + Body + 'static,
    T::Data: Send,
    T::Error: Into<Box<dyn serde::ser::StdError + Send + Sync>>,
{
    pub connection_pool: Pool<ConnectionManager<T>>,
    pub uri: String,
    pub _socket_addr: SocketAddr,
}

impl<T> TargetConnectionPool<T>
where
    T: Send + Sync + Body + 'static,
    T::Data: Send,
    T::Error: Into<Box<dyn serde::ser::StdError + Send + Sync>>,
{
    pub async fn create_health_check_pool(
        &self,
    ) -> Result<TargetConnectionPool<Empty<Bytes>>, TargetConnectionPoolCloneError> {
        Ok(TargetConnectionPool::<Empty<Bytes>> {
            connection_pool: Pool::builder()
                .max_size(1)
                .build(ConnectionManager::new(self._socket_addr))
                .await
                .map_err(TargetConnectionPoolCloneError::CreateNewPool)?,
            uri: self.uri.clone(),
            _socket_addr: self._socket_addr,
        })
    }
}

#[derive(Debug, thiserror::Error)]
pub enum TargetConnectionPoolCloneError {
    #[error("Failed to create new pool, error: {0}")]
    CreateNewPool(ConnectionManagerError),
}

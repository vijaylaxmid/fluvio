// JS Wrapper for ReplicaLeader

use std::sync::Arc;

use log::{debug, trace};

use futures::stream::StreamExt;

use flv_client::SpuLeader;
use flv_client::ReplicaLeader;
use flv_future_aio::sync::RwLock;
use flv_future_core::run_block_on;
use flv_client::ClientError;

use kf_protocol::api::MAX_BYTES;
use kf_protocol::api::Isolation;
use kf_protocol::api::PartitionOffset;

use node_bindgen::derive::node_bindgen;
use node_bindgen::core::NjError;
use node_bindgen::core::val::JsEnv;
use node_bindgen::core::TryIntoJs;
use node_bindgen::sys::napi_value;
use node_bindgen::core::JSClass;

use types::socket_helpers::ServerAddress;

type SharedSpuLeader = Arc<RwLock<SpuLeader>>;
pub struct ReplicaLeaderWrapper(SpuLeader);

impl From<SpuLeader> for ReplicaLeaderWrapper {
    fn from(leader: SpuLeader) -> Self {
        Self(leader)
    }
}

impl TryIntoJs for ReplicaLeaderWrapper {
    fn try_to_js(self, js_env: &JsEnv) -> Result<napi_value, NjError> {
        let new_instance = JsReplicaLeader::new_instance(js_env, vec![])?;
        JsReplicaLeader::unwrap(js_env, new_instance)?.set_leader(self.0);
        Ok(new_instance)
    }
}

pub struct JsReplicaLeader {
    inner: Option<SharedSpuLeader>,
}

#[node_bindgen]
impl JsReplicaLeader {
    #[node_bindgen(constructor)]
    pub fn new() -> Self {
        Self { inner: None }
    }

    pub fn set_leader(&mut self, leader: SpuLeader) {
        self.inner.replace(Arc::new(RwLock::new(leader)));
    }

    fn get_addr(&self) -> Option<ServerAddress> {
        // since SpuLeader is locked, we need to read in order to access it
        self.inner.as_ref().map_or(None, move |c| {
            run_block_on(async move {
                let c1 = c.clone();
                let read_client = c1.read().await;
                Some(read_client.addr().clone())
            })
        })
    }

    /// JS method to return Replica Leader host:port address in string format
    #[node_bindgen]
    fn full_address(&self) -> String {
        if let Some(result) = self.get_addr() {
            result.to_string()
        } else {
            "".to_owned()
        }
    }

    /// JS method to send message to the replica leader
    #[node_bindgen]
    async fn produce(&self, message: String) -> Result<i64, ClientError> {
        let leader = self.inner.as_ref().unwrap().clone();

        let mut producer = leader.write().await;
        let bytes = message.into_bytes();
        let len = bytes.len();
        producer.send_record(bytes).await.map(|_| len as i64)
    }

    /// JS method to consume message from replica leader
    /// * consume(replica_leader, emitter_cb)
    #[node_bindgen]
    async fn consume<F: Fn(String, String)>(&self, cb: F) {
        // there should be always leader
        let leader = self.inner.as_ref().unwrap().clone();

        let mut leader_w = leader.write().await;

        let offsets = leader_w
            .fetch_offsets()
            .await
            .expect("fetch offsets should not fail");
        let beginning_offset = offsets.start_offset();

        /*
        TODO: bring back when JSON support is available.

        let beginning_offset = if from_beginning {
            offsets.start_offset()
        } else {
            offsets.last_stable_offset()
        };
        */

        debug!("start consumer fetch from offset: {}", beginning_offset);

        let mut log_stream =
            leader_w.fetch_logs(beginning_offset, MAX_BYTES, Isolation::ReadCommitted);

        let event = "data".to_owned();

        while let Some(partition_response) = log_stream.next().await {
            let records = partition_response.records;

            trace!("received records: {:#?}", records);

            for batch in records.batches {
                for record in batch.records {
                    if let Some(bytes) = record.value().inner_value() {
                        let msg = String::from_utf8(bytes).expect("string");
                        trace!("msg: {}", msg);

                        cb(event.clone(), msg);
                    }
                }
            }
        }
    }
}

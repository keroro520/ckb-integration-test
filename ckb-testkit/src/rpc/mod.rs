mod id_generator;
#[macro_use]
mod macros;
mod error;
mod v2019;
mod v2021;

use ckb_error::AnyError;
// TODO replace json types with core types
use ckb_jsonrpc_types::{
    Alert, BannedAddr, Block, BlockTemplate, BlockView, CellWithStatus, ChainInfo, Consensus,
    DryRunResult, EpochView, HeaderView, LocalNode, OutPoint, RawTxPool, RemoteNode, Timestamp,
    Transaction, TransactionWithStatus, TxPoolInfo,
};
use ckb_types::core::{
    BlockNumber as CoreBlockNumber, Capacity as CoreCapacity, EpochNumber as CoreEpochNumber,
    Version as CoreVersion,
};
use ckb_types::{packed::Byte32, prelude::*};
use lazy_static::lazy_static;
use v2019::Inner2019;
use v2021::Inner2021;

lazy_static! {
    pub static ref HTTP_CLIENT: reqwest::blocking::Client = reqwest::blocking::Client::builder()
        .timeout(::std::time::Duration::from_secs(30))
        .build()
        .expect("reqwest Client build");
}

macro_rules! item2019_to_item2021 {
    ($item2019:expr) => {{
        let raw2019 = serde_json::to_string(&$item2019).unwrap();
        let raw2021 = raw2019
            // interfaces that includes block header
            .replace("uncles_hash", "extra_hash")
            // get_consensus
            .replace(
                "\"permanent_difficulty_in_dummy\":",
                "\"hardfork_features\":[],\"permanent_difficulty_in_dummy\":",
            );
        serde_json::from_str(&raw2021).unwrap()
    }};
}

macro_rules! item2021_to_item2019 {
    ($item2021:expr) => {{
        let raw2021 = serde_json::to_string(&$item2021).unwrap();
        let raw2019 = raw2021.replace("extra_hash", "uncles_hash");
        serde_json::from_str(&raw2019).unwrap()
    }};
}

pub struct RpcClient {
    pub ckb2021: bool,
    inner2019: Inner2019,
    inner2021: Inner2021,
}

impl Clone for RpcClient {
    fn clone(&self) -> RpcClient {
        RpcClient::new(self.inner2021.url.as_str(), self.ckb2021)
    }
}

impl RpcClient {
    pub fn new(uri: &str, ckb2021: bool) -> Self {
        Self {
            inner2019: Inner2019::new(uri),
            inner2021: Inner2021::new(uri),
            ckb2021,
        }
    }

    pub fn url(&self) -> &str {
        self.inner2021.url.as_ref()
    }

    pub fn inner(&self) -> &Inner2021 {
        &self.inner2021
    }

    pub fn get_block(&self, hash: Byte32) -> Option<BlockView> {
        if self.ckb2021 {
            self.inner2021
                .get_block(hash.unpack())
                .expect("rpc call get_block")
        } else {
            item2019_to_item2021!(self
                .inner2019
                .get_block(hash.unpack())
                .expect("rpc call get_block"))
        }
    }

    pub fn get_fork_block(&self, hash: Byte32) -> Option<BlockView> {
        if self.ckb2021 {
            self.inner2021
                .get_fork_block(hash.unpack())
                .expect("rpc call get_fork_block")
        } else {
            item2019_to_item2021!(self
                .inner2019
                .get_fork_block(hash.unpack())
                .expect("rpc call get_fork_block"))
        }
    }

    pub fn get_block_by_number(&self, number: CoreBlockNumber) -> Option<BlockView> {
        if self.ckb2021 {
            self.inner2021
                .get_block_by_number(number.into())
                .expect("rpc call get_block_by_number")
        } else {
            item2019_to_item2021!(self
                .inner2019
                .get_block_by_number(number.into())
                .expect("rpc call get_block_by_number"))
        }
    }

    pub fn get_header(&self, hash: Byte32) -> Option<HeaderView> {
        if self.ckb2021 {
            self.inner2021
                .get_header(hash.unpack())
                .expect("rpc call get_header")
        } else {
            item2019_to_item2021!(self
                .inner2019
                .get_header(hash.unpack())
                .expect("rpc call get_header"))
        }
    }

    pub fn get_header_by_number(&self, number: CoreBlockNumber) -> Option<HeaderView> {
        if self.ckb2021 {
            self.inner2021
                .get_header_by_number(number.into())
                .expect("rpc call get_header_by_number")
        } else {
            item2019_to_item2021!(self
                .inner2019
                .get_header_by_number(number.into())
                .expect("rpc call get_header_by_number"))
        }
    }

    pub fn get_transaction(&self, hash: Byte32) -> Option<TransactionWithStatus> {
        if self.ckb2021 {
            self.inner2021
                .get_transaction(hash.unpack())
                .expect("rpc call get_transaction")
        } else {
            item2019_to_item2021!(self
                .inner2019
                .get_transaction(hash.unpack())
                .expect("rpc call get_transaction"))
        }
    }

    pub fn get_block_hash(&self, number: CoreBlockNumber) -> Option<Byte32> {
        self.inner()
            .get_block_hash(number.into())
            .expect("rpc call get_block_hash")
            .map(|x| x.pack())
    }

    pub fn get_tip_header(&self) -> HeaderView {
        if self.ckb2021 {
            self.inner2021
                .get_tip_header()
                .expect("rpc call get_block_hash")
        } else {
            item2019_to_item2021!(self
                .inner2019
                .get_tip_header()
                .expect("rpc call get_block_hash"))
        }
    }

    pub fn get_live_cell(&self, out_point: OutPoint, with_data: bool) -> CellWithStatus {
        if self.ckb2021 {
            self.inner2021
                .get_live_cell(out_point, with_data)
                .expect("rpc call get_live_cell")
        } else {
            let out_point = item2019_to_item2021!(out_point);
            item2019_to_item2021!(self
                .inner2019
                .get_live_cell(out_point, with_data)
                .expect("rpc call get_live_cell"))
        }
    }

    pub fn get_tip_block_number(&self) -> CoreBlockNumber {
        self.inner()
            .get_tip_block_number()
            .expect("rpc call get_tip_block_number")
            .into()
    }

    pub fn get_current_epoch(&self) -> EpochView {
        self.inner()
            .get_current_epoch()
            .expect("rpc call get_current_epoch")
    }

    pub fn get_epoch_by_number(&self, number: CoreEpochNumber) -> Option<EpochView> {
        self.inner()
            .get_epoch_by_number(number.into())
            .expect("rpc call get_epoch_by_number")
    }

    pub fn get_consensus(&self) -> Consensus {
        if self.ckb2021 {
            self.inner2021
                .get_consensus()
                .expect("rpc call get_consensus")
        } else {
            item2019_to_item2021!(self
                .inner2019
                .get_consensus()
                .expect("rpc call get_consensus"))
        }
    }

    pub fn local_node_info(&self) -> LocalNode {
        self.inner()
            .local_node_info()
            .expect("rpc call local_node_info")
    }

    pub fn get_peers(&self) -> Vec<RemoteNode> {
        self.inner().get_peers().expect("rpc call get_peers")
    }

    pub fn get_banned_addresses(&self) -> Vec<BannedAddr> {
        self.inner()
            .get_banned_addresses()
            .expect("rpc call get_banned_addresses")
    }

    pub fn set_ban(
        &self,
        address: String,
        command: String,
        ban_time: Option<Timestamp>,
        absolute: Option<bool>,
        reason: Option<String>,
    ) {
        self.inner()
            .set_ban(address, command, ban_time, absolute, reason)
            .expect("rpc call set_ban")
    }

    pub fn get_block_template(
        &self,
        bytes_limit: Option<u64>,
        proposals_limit: Option<u64>,
        max_version: Option<CoreVersion>,
    ) -> BlockTemplate {
        if self.ckb2021 {
            let bytes_limit = bytes_limit.map(Into::into);
            let proposals_limit = proposals_limit.map(Into::into);
            let max_version = max_version.map(Into::into);
            self.inner2021
                .get_block_template(bytes_limit, proposals_limit, max_version)
                .expect("rpc call get_block_template2021")
        } else {
            let bytes_limit = bytes_limit.map(Into::into);
            let proposals_limit = proposals_limit.map(Into::into);
            let max_version = max_version.map(Into::into);
            item2019_to_item2021!(self
                .inner2019
                .get_block_template(bytes_limit, proposals_limit, max_version)
                .expect("rpc call get_block_template2019"))
        }
    }

    pub fn submit_block(&self, work_id: String, block: Block) -> Result<Byte32, AnyError> {
        if self.ckb2021 {
            self.inner2021
                .submit_block(work_id, block)
                .map(|x| x.pack())
        } else {
            let block2019 = item2021_to_item2019!(&block);
            self.inner2019
                .submit_block(work_id, block2019)
                .map(|x| x.pack())
        }
    }

    pub fn get_blockchain_info(&self) -> ChainInfo {
        self.inner()
            .get_blockchain_info()
            .expect("rpc call get_blockchain_info")
    }

    pub fn get_block_median_time(&self, block_hash: Byte32) -> Option<Timestamp> {
        self.inner()
            .get_block_median_time(block_hash.unpack())
            .expect("rpc call get_block_median_time")
    }

    pub fn send_transaction(&self, tx: Transaction) -> Byte32 {
        self.send_transaction_result(tx)
            .expect("rpc call send_transaction")
    }

    pub fn send_transaction_result(&self, tx: Transaction) -> Result<Byte32, AnyError> {
        if self.ckb2021 {
            let ret = self
                .inner2021
                .send_transaction(tx, Some("passthrough".to_string()));

            // NOTE: This if-statement is a workaround to tx-pool async excute transaction's scripts
            // and response RPC request. Even after returning `Ok(hash)`, the transaction's scripts
            // may not been executed yet.
            if let Ok(ref hash) = ret {
                loop {
                    if let Some(txstatus) = self.inner2021.get_transaction(hash.clone()).unwrap() {
                        if txstatus.tx_status.status != ckb_jsonrpc_types::Status::Unknown {
                            break;
                        }
                    }
                }
            }

            ret.map(|h256| h256.pack())
        } else {
            let tx = item2021_to_item2019!(tx);
            self.inner2019
                .send_transaction(tx, Some("passthrough".to_string()))
                .map(|h256| h256.pack())
        }
    }

    pub fn dry_run_transaction(&self, tx: Transaction) -> DryRunResult {
        if self.ckb2021 {
            self.inner2021
                .dry_run_transaction(tx)
                .expect("rpc call dry_run_transaction")
        } else {
            let tx = item2019_to_item2021!(tx);
            item2019_to_item2021!(self
                .inner2019
                .dry_run_transaction(tx)
                .expect("rpc call dry_run_transaction"))
        }
    }

    pub fn send_alert(&self, alert: Alert) {
        self.inner().send_alert(alert).expect("rpc call send_alert")
    }

    pub fn tx_pool_info(&self) -> TxPoolInfo {
        self.inner().tx_pool_info().expect("rpc call tx_pool_info")
    }

    pub fn add_node(&self, peer_id: String, address: String) {
        self.inner()
            .add_node(peer_id, address)
            .expect("rpc call add_node");
    }

    pub fn remove_node(&self, peer_id: String) {
        self.inner()
            .remove_node(peer_id)
            .expect("rpc call remove_node")
    }

    pub fn truncate(&self, target_tip_hash: Byte32) {
        self.inner()
            .truncate(target_tip_hash.unpack())
            .expect("rpc call truncate")
    }

    pub fn calculate_dao_maximum_withdraw(
        &self,
        out_point: OutPoint,
        hash: Byte32,
    ) -> CoreCapacity {
        self.inner()
            .calculate_dao_maximum_withdraw(out_point, hash.unpack())
            .expect("rpc call calculate_dao_maximum_withdraw")
            .into()
    }

    pub fn process_block_without_verify(
        &self,
        block: Block,
        should_broadcast: bool,
    ) -> Option<Byte32> {
        if self.ckb2021 {
            self.inner2021
                .process_block_without_verify(block, should_broadcast)
                .expect("rpc call process_block_without_verify")
                .map(|h256| h256.pack())
        } else {
            let block = item2021_to_item2019!(block);
            self.inner2019
                .process_block_without_verify(block, should_broadcast)
                .expect("rpc call process_block_without_verify")
                .map(|h256| h256.pack())
        }
    }

    pub fn calculate_dao_field(&self, block_template: BlockTemplate) -> Result<Byte32, AnyError> {
        assert!(self.ckb2021);
        self.inner2021
            .calculate_dao_field(block_template)
            .map(Into::into)
    }

    pub fn get_raw_tx_pool(&self, verbose: Option<bool>) -> Result<RawTxPool, AnyError> {
        assert!(self.ckb2021);
        self.inner2021.get_raw_tx_pool(verbose)
    }
}

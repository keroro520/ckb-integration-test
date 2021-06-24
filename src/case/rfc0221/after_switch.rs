use crate::case::rfc0221::util::{committed_timestamp, median_timestamp};
use crate::case::{Case, CaseOptions};
use crate::node::NodeOptions;
use crate::nodes::Nodes;
use crate::util::since_from_relative_timestamp;
use crate::CKB_FORK2021_BINARY;
use ckb_types::core::EpochNumber;
use ckb_types::{
    core::TransactionBuilder,
    packed::{CellInput, CellOutput},
    prelude::*,
};
use std::thread::sleep;
use std::time::Duration;

const RFC0221_EPOCH_NUMBER: EpochNumber = 3;

pub struct RFC0221AfterSwitch;

impl Case for RFC0221AfterSwitch {
    fn case_options(&self) -> CaseOptions {
        CaseOptions {
            make_all_nodes_connected: true,
            make_all_nodes_synced: true,
            make_all_nodes_connected_and_synced: true,
            node_options: vec![NodeOptions {
                node_name: "ckb-fork2021",
                ckb_binary: CKB_FORK2021_BINARY.lock().clone(),
                initial_database: "db/Epoch2V2TestData",
                chain_spec: "spec/ckb-fork2021",
                app_config: "config/ckb-fork2021",
            }]
            .into_iter()
            .collect(),
        }
    }

    fn run(&self, nodes: Nodes) {
        let node_v2 = nodes.get_node("ckb-fork2021");

        // Move the chain to height = rfc0221_switch + 37
        {
            let mut over_move_switch_cnt = 37;
            loop {
                if !is_rfc0221_switched(node_v2.rpc_client().get_current_epoch().number.value()) {
                    node_v2.mine(1);
                } else if over_move_switch_cnt > 0 {
                    over_move_switch_cnt -= 1;
                    node_v2.mine(1);
                    sleep(Duration::from_secs(1));
                } else {
                    break;
                }
            }
        }

        // Construct a transaction tx:
        //   - since: relative 2 seconds
        let relative_secs = 2;
        let relative_mills = relative_secs * 1000;
        let since = since_from_relative_timestamp(relative_secs);
        let input = &{
            // Use the last live cell as input to make sure the constructed
            // transaction cannot pass the "since verification" at short future
            node_v2
                .get_live_always_success_cells()
                .pop()
                .expect("last live cell")
        };
        let input_block_number = input
            .transaction_info
            .as_ref()
            .expect("live cell should have transaction info")
            .block_number;
        let start_time_of_rfc0221 = committed_timestamp(node_v2, input_block_number);
        let tx = TransactionBuilder::default()
            .input(CellInput::new(input.out_point.clone(), since))
            .output(
                CellOutput::new_builder()
                    .lock(input.cell_output.lock())
                    .type_(input.cell_output.type_())
                    .capacity(input.capacity().pack())
                    .build(),
            )
            .output_data(Default::default())
            .cell_dep(node_v2.always_success_cell_dep())
            .build();

        loop {
            let tip_number = node_v2.get_tip_block_number();
            let tip_median_time = median_timestamp(node_v2, tip_number);
            crate::debug!("tip_number = {}", tip_number);
            crate::debug!("tip_median_time = {}", tip_median_time);
            crate::debug!(
                "input_median_time + relative_mills = {}",
                median_timestamp(node_v2, input_block_number) + relative_mills
            );
            crate::debug!(
                "start_time_of_rfc0221 + relative_mills = {}",
                start_time_of_rfc0221 + relative_mills
            );
            crate::debug!(
                "start_time_of_rfc0221 - relative_mills - tip_median_time = {}",
                start_time_of_rfc0221 - relative_mills - tip_median_time
            );
            if start_time_of_rfc0221 + relative_mills <= tip_median_time {
                break;
            } else {
                let result = node_v2
                    .rpc_client()
                    .send_transaction_result(tx.pack().data().into());
                assert!(
                    result.is_err(),
                    "After RFC0221, node_v2 should reject tx according to rfc0221, but got: {:?}",
                    result,
                );
            }

            sleep(Duration::from_secs(1));
            node_v2.mine(1);
        }

        let sent = node_v2
            .rpc_client()
            .send_transaction_result(tx.pack().data().into());
        assert!(
            sent.is_ok(),
            "After RFC0221, node_v2 should accept tx according to rfc0221, but got: {:?}",
            sent,
        );
    }
}

fn is_rfc0221_switched(epoch_number: EpochNumber) -> bool {
    epoch_number >= RFC0221_EPOCH_NUMBER
}
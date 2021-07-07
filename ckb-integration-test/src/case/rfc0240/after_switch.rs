// ## Cases And Expect Results
//
// After rfc0240, transactions are allowed to reference any on-chain `HeaderDep`

use crate::case::{Case, CaseOptions};
use crate::CKB2021;
use ckb_jsonrpc_types::EpochNumberWithFraction;
use ckb_testkit::node::{Node, NodeOptions};
use ckb_testkit::nodes::Nodes;
use ckb_testkit::util::wait_until;
use ckb_types::core::EpochNumber;
use ckb_types::prelude::Pack;

const RFC0240_EPOCH_NUMBER: EpochNumber = 3;

pub struct RFC0240AfterSwitch;

impl Case for RFC0240AfterSwitch {
    fn case_options(&self) -> CaseOptions {
        CaseOptions {
            make_all_nodes_connected: true,
            make_all_nodes_synced: true,
            make_all_nodes_connected_and_synced: true,
            node_options: vec![
                NodeOptions {
                    node_name: "node2021",
                    ckb_binary: CKB2021.read().unwrap().clone(),
                    initial_database: "testdata/db/Epoch2V2TestData",
                    chain_spec: "testdata/spec/ckb2021",
                    app_config: "testdata/config/ckb2021",
                },
                NodeOptions {
                    node_name: "node2021_2",
                    ckb_binary: CKB2021.read().unwrap().clone(),
                    initial_database: "testdata/db/Epoch2V2TestData",
                    chain_spec: "testdata/spec/ckb2021",
                    app_config: "testdata/config/ckb2021",
                },
            ]
            .into_iter()
            .collect(),
        }
    }

    fn run(&self, nodes: Nodes) {
        let node2021 = nodes.get_node("node2021");
        while !is_rfc0240_switched(node2021) {
            node2021.mine(100);
        }
        assert!(is_rfc0240_switched(node2021));
        assert!(node2021.consensus().cellbase_maturity > EpochNumberWithFraction::from(0));

        let tip_hash = node2021.get_tip_block().hash();
        let tx = {
            let input = node2021.get_live_always_success_cells()[0].to_owned();
            let tx = node2021.always_success_transaction(&input);
            tx.as_advanced_builder().header_dep(tip_hash).build()
        };
        let result = node2021
            .rpc_client()
            .send_transaction_result(tx.pack().data().into());
        assert!(
            result.is_ok(),
            "after rfc0240, {} should accept tx, but got: {:?}",
            node2021.node_name(),
            result,
        );

        let tx_relayed = wait_until(30, || {
            nodes.nodes().all(|node| node.is_transaction_pending(&tx))
        });
        assert!(tx_relayed, "tx should be relayed to all nodes");

        node2021.mine(node2021.consensus().tx_proposal_window.closest.value() + 1);
        assert!(node2021.is_transaction_committed(&tx));
    }
}

fn is_rfc0240_switched(node: &Node) -> bool {
    node.rpc_client().get_current_epoch().number.value() >= RFC0240_EPOCH_NUMBER
}